// Run with: node --test crates/tanoshi-web/scripts/queue-repro.test.cjs
const assert = require("node:assert/strict");
const { webcrypto } = require("node:crypto");
const fs = require("node:fs");
const { performance } = require("node:perf_hooks");
const { test } = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(`${__dirname}/queue-repro.js`, "utf8");
const storageKey = "tanoshi-queue-repro-run";
const flush = () => new Promise(setImmediate);

function harness(port) {
  const storage = new Map([["token", "test-only-token"]]);
  const timers = new Map();
  const calls = [];
  const cancellations = [];
  const logs = [];
  const state = {
    running: false, queue: [], hold: false, active: 0, maximumActive: 0,
    loseSeedResponse: false, loseCleanupResponse: false,
  };
  const window = {
    location: { href: "https://reader.example.test/library" },
    __TANOSHI_PORT__: port,
    crypto: webcrypto,
    localStorage: {
      getItem: (key) => storage.get(key) ?? null,
      setItem: (key, value) => storage.set(key, value),
      removeItem: (key) => storage.delete(key),
    },
    setTimeout: (callback, milliseconds) => {
      const handle = {};
      timers.set(handle, { callback, milliseconds });
      return handle;
    },
    clearTimeout: (handle) => timers.delete(handle),
  };
  const nativeFetch = async function(input, options) {
    assert.equal(this, window);
    calls.push({ input, options });
    if (!options?.body) return { ok: true };
    assert.equal(input, port ? `http://localhost:${port}/graphql` : "https://reader.example.test/graphql");
    assert.equal(options.headers.Authorization, "Bearer test-only-token");
    const { query, variables } = JSON.parse(options.body);
    let data;
    if (query.includes("seedDownloadQueueRepro")) {
      assert.equal(JSON.parse(storage.get(storageKey)).runId, variables.runId,
        "store recovery information before sending the seed request");
      const title = `Queue repro - ${variables.runId}`;
      const existing = state.queue.filter((chapter) => chapter.mangaTitle === title);
      if (!existing.length) {
        state.queue.push(...Array.from({ length: variables.chapters }, (_, index) => ({
          chapterId: -10_000 - index, sourceId: -1, sourceName: "Tanoshi queue repro", mangaTitle: title,
        })));
      }
      if (state.loseSeedResponse) {
        state.loseSeedResponse = false;
        throw new Error("lost seed response");
      }
      data = { seedDownloadQueueRepro: state.queue.filter((chapter) => chapter.mangaTitle === title)
        .map((chapter) => chapter.chapterId) };
    } else if (query.includes("removeChaptersFromQueue")) {
      assert.equal(query, "mutation QueueRepro($ids: [Int!]!) { removeChaptersFromQueue(ids: $ids) }");
      let release;
      const gate = new Promise((resolve) => { release = resolve; });
      cancellations.push({ ids: variables.ids, release });
      state.maximumActive = Math.max(state.maximumActive, ++state.active);
      if (state.hold) await gate;
      state.active--;
      state.queue = state.queue.filter((chapter) => !variables.ids.includes(chapter.chapterId));
      data = { removeChaptersFromQueue: variables.ids.length };
    } else if (query.includes("clearDownloadQueueRepro")) {
      const title = `Queue repro - ${variables.runId}`;
      const count = state.queue.filter((chapter) => chapter.mangaTitle === title).length;
      state.queue = state.queue.filter((chapter) => chapter.mangaTitle !== title);
      if (state.loseCleanupResponse) {
        state.loseCleanupResponse = false;
        throw new Error("lost cleanup response");
      }
      data = { clearDownloadQueueRepro: count };
    } else {
      data = { downloadStatus: state.running, downloadQueue: state.queue };
    }
    return { ok: true, json: async () => ({ data }) };
  };
  window.fetch = nativeFetch;
  const context = vm.createContext({ window, URL, performance, console:
    Object.fromEntries(["info", "error", "table"].map((name) => [name, (...args) => logs.push(args)])) });
  const paste = () => vm.runInContext(source, context);
  paste();
  return { window, state, storage, timers, calls, cancellations, logs, nativeFetch, paste,
    get api() { return window.tanoshiQueueRepro; } };
}

for (const port of [undefined, 4321]) {
  test(`cancel only this run's dummy IDs through five parallel requests (${port ? "Tauri" : "web"})`, async () => {
    const app = harness(port);
    assert.equal(app.calls.length, 0, "pasting must not send any requests");
    app.state.queue = [{ chapterId: 1, sourceId: 123, sourceName: "Real source", mangaTitle: "Real manga" }];
    const seeded = await app.api.seed(5000, 30);
    assert.equal(seeded.length, 5000);
    const run = JSON.parse(app.storage.get(storageKey));
    const title = `Queue repro - ${run.runId}`;
    // Include another test run and entries that match only part of the marker.
    const untouched = [app.state.queue[0],
      { chapterId: -3, sourceId: -1, sourceName: "Tanoshi queue repro", mangaTitle: "Queue repro - another run" },
      { chapterId: 2, sourceId: -1, sourceName: "Tanoshi queue repro", mangaTitle: title },
      { chapterId: -4, sourceId: 123, sourceName: "Tanoshi queue repro", mangaTitle: title },
      { chapterId: -5, sourceId: -1, sourceName: "Real source", mangaTitle: title },
    ];
    app.state.queue.unshift(...untouched.slice(1));
    app.state.hold = true;
    app.paste(); // Recover the run after re-pasting/reloading the helper.
    const operation = app.api.cancelFront(100);
    await flush();
    assert.equal(app.state.maximumActive, 5);
    assert.equal(app.cancellations.length, 5);
    assert.ok(app.cancellations.every(({ ids }) => ids.length === 20));
    assert.deepEqual(app.cancellations.flatMap(({ ids }) => ids).sort((a, b) => b - a),
      Array.from(seeded).slice(0, 100));
    app.paste(); // A replacement helper must keep the in-flight operation guard.
    await assert.rejects(app.api.cleanup(), /current queue test operation/);
    app.cancellations.forEach(({ release }) => release());
    const settled = await operation;
    assert.ok(settled.every((result) => result.status === "fulfilled"));
    assert.ok(untouched.every((chapter) => app.state.queue.includes(chapter)));
    assert.ok(!JSON.stringify(app.logs).includes("test-only-token"));
  });
}

test("cleanup recovers after lost seed and cleanup responses", async () => {
  const app = harness();
  app.state.queue = [{ chapterId: 1, mangaTitle: "Real manga" }];
  app.state.loseSeedResponse = true;
  await assert.rejects(app.api.seed(10, 2), /lost seed response/);
  assert.ok(app.storage.has(storageKey));
  app.paste();
  const ids = await app.api.seed(10, 2);
  assert.equal(ids.length, 10);
  assert.equal(app.state.queue.length, 11, "retry must not seed duplicate entries");
  app.state.loseCleanupResponse = true;
  await assert.rejects(app.api.cleanup(), /lost cleanup response/);
  assert.ok(app.storage.has(storageKey), "keep recovery information after failed cleanup");
  app.paste();
  assert.equal(await app.api.cleanup(), 0);
  assert.equal(app.storage.has(storageKey), false);
  assert.deepEqual(app.state.queue, [{ chapterId: 1, mangaTitle: "Real manga" }]);
});

test("refuse real-queue cancellations, invalid sizes, running downloads, and insufficient dummy entries", async () => {
  const app = harness();
  await assert.rejects(app.api.cancelFront(5), /seed/);
  await assert.rejects(app.api.cleanup(), /seed/);
  for (const chapters of [0, 5001, 1.5]) await assert.rejects(app.api.seed(chapters), /Chapter count/);
  for (const pages of [0, 101, NaN]) await assert.rejects(app.api.seed(10, pages), /Page count/);
  assert.equal(app.calls.length, 0);
  app.state.running = true;
  await assert.rejects(app.api.seed(10, 2), /Pause downloads/);
  assert.equal(app.storage.has(storageKey), false);
  app.state.running = false;
  await app.api.seed(10, 2);
  await assert.rejects(app.api.seed(11, 2), /cleanup/);
  await assert.rejects(app.api.cancelFront(11), /10 dummy chapters/);
  app.state.running = true;
  await assert.rejects(app.api.cancelFront(5), /Pause downloads/);
  assert.equal(app.cancellations.length, 0);
});

test("simulated GraphQL delay remains reversible and leaves other requests alone", async () => {
  const app = harness();
  app.api.slowResponses(3000);
  const delayed = app.window.fetch({ url: "http://localhost:4321/graphql" });
  let otherDone = false;
  const other = app.window.fetch("/image").then(() => { otherDone = true; });
  await flush();
  assert.equal(otherDone, true);
  assert.equal(app.timers.size, 1);
  app.api.restore();
  await Promise.all([delayed, other]);
  assert.equal(app.timers.size, 0);
  assert.equal(app.window.fetch, app.nativeFetch);
});
