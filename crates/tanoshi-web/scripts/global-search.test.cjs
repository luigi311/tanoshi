// Build first: cd crates/tanoshi-web && trunk build --release
// Run: node --test crates/tanoshi-web/scripts/global-search.test.cjs
// Optional: TANOSHI_WEB_DIST=/path/to/dist CHROMIUM=/path/to/chromium
const assert = require("node:assert/strict");
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const { test } = require("node:test");

const dist = path.resolve(process.env.TANOSHI_WEB_DIST || path.join(__dirname, "../dist"));
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

test("global search bounds requests, shows completed sources, and cancels replaced searches", { timeout: 60000 }, async (t) => {
  assert.ok(fs.existsSync(path.join(dist, "index.html")), "build tanoshi-web before running this test");
  const sources = Array.from({ length: 16 }, (_, index) => ({
    id: index + 1, name: `Source ${index + 1}`, version: "test", icon: "",
  }));
  const requests = [];
  const respond = (response, body) => {
    response.writeHead(200, { "Content-Type": "application/json", "Cache-Control": "no-store" });
    response.end(JSON.stringify(body));
  };
  const server = http.createServer((request, response) => {
    const pathname = new URL(request.url, "http://fixture").pathname;
    if (pathname === "/graphql") {
      let body = "";
      request.on("data", (chunk) => { body += chunk; });
      request.on("end", () => {
        const { operationName, variables } = JSON.parse(body);
        if (operationName === "FetchServerStatus") {
          respond(response, { data: { serverStatus: { activated: true, loggedin: true, version: "test" } } });
        } else if (operationName === "FetchSources") {
          respond(response, { data: { installedSources: sources } });
        } else if (operationName === "BrowseSource") {
          const { sourceId: id, query: keyword, page } = variables;
          assert.equal(page, 1);
          const record = { id, keyword, settled: false };
          const settle = () => {
            record.settled = true;
          };
          response.once("finish", settle);
          response.once("close", settle);
          record.reply = (fail = false) => {
            if (response.destroyed || response.writableEnded) return;
            respond(response, fail
              ? { errors: [{ message: `Source ${id} failed` }] }
              : { data: {
                source: { name: `Source ${id}` },
                browseSource: [{ id: 0, path: `/manga/${id}`, title: `${keyword} source ${id}`,
                  coverUrl: "", isFavorite: false }],
              } });
          };
          requests.push(record);
        } else {
          respond(response, { errors: [{ message: `Unexpected operation ${operationName}` }] });
        }
      });
      return;
    }
    if (pathname === "/sw.js") {
      response.writeHead(200, { "Content-Type": "text/javascript" });
      response.end("");
      return;
    }
    if (pathname.startsWith("/image/")) {
      response.writeHead(404);
      response.end();
      return;
    }
    let file = path.join(dist, pathname);
    if (!fs.existsSync(file) || !fs.statSync(file).isFile()) file = path.join(dist, "index.html");
    const mime = { ".js": "text/javascript", ".wasm": "application/wasm", ".html": "text/html",
      ".css": "text/css", ".png": "image/png" };
    response.writeHead(200, { "Content-Type": mime[path.extname(file)] || "application/octet-stream",
      "Cache-Control": "no-store" });
    fs.createReadStream(file).pipe(response);
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const origin = `http://127.0.0.1:${server.address().port}`;
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), "tanoshi-global-search-test-"));
  const browser = spawn(process.env.CHROMIUM || "chromium", [
    "--headless", "--disable-gpu", "--no-first-run", "--remote-debugging-pipe", `--user-data-dir=${profile}`,
  ], { stdio: ["ignore", "ignore", "pipe", "pipe", "pipe"] });
  t.after(async () => {
    if (browser.pid && browser.exitCode === null && browser.signalCode === null) {
      const exited = new Promise((resolve) => browser.once("exit", resolve));
      browser.kill();
      await exited;
    }
    server.closeAllConnections();
    server.close();
    fs.rmSync(profile, { recursive: true, force: true });
  });
  let sequence = 0;
  let buffer = "";
  let stderr = "";
  const pending = new Map();
  browser.stderr.on("data", (chunk) => { stderr += chunk; });
  browser.stdio[4].on("data", (chunk) => {
    buffer += chunk;
    let end;
    while ((end = buffer.indexOf("\0")) >= 0) {
      const raw = buffer.slice(0, end);
      buffer = buffer.slice(end + 1);
      if (!raw) continue;
      const message = JSON.parse(raw);
      if (pending.has(message.id)) {
        const { resolve, reject } = pending.get(message.id);
        pending.delete(message.id);
        message.error ? reject(Error(JSON.stringify(message.error))) : resolve(message.result);
      }
    }
  });
  const call = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const id = ++sequence;
    pending.set(id, { resolve, reject });
    browser.stdio[3].write(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }) + "\0");
  });
  const { targetId } = await call("Target.createTarget", { url: "about:blank" });
  const { sessionId } = await call("Target.attachToTarget", { targetId, flatten: true });
  const rpc = (method, params = {}) => call(method, params, sessionId);
  const evaluate = async (expression) => {
    const result = await rpc("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (result.exceptionDetails) throw Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  const until = async (check, label) => {
    for (let index = 0; index < 100; ++index) {
      if (await check()) return;
      await delay(50);
    }
    throw Error(`${label}: ${JSON.stringify(requests.map(({ id, keyword }) => ({ id, keyword })))} ${stderr.slice(-1000)}`);
  };
  const requested = (keyword) => requests.filter((request) => request.keyword === keyword);
  const dispatchedIds = (keyword) => evaluate(`window.__searchRequests
    .filter(request => request.keyword === ${JSON.stringify(keyword)})
    .map(request => request.id).sort((a, b) => a - b)`);
  const sourceRequest = (keyword, id) => requested(keyword).find((request) => request.id === id);
  const reply = (keyword, id, fail = false) => {
    const request = sourceRequest(keyword, id);
    assert.ok(request, `Source ${id} has not started for ${keyword}`);
    request.reply(fail);
  };
  const visible = (keyword, id) => evaluate(`[...document.querySelectorAll('span')]
    .some(span => span.textContent === ${JSON.stringify(`${keyword} source ${id}`)})`);
  const search = (keyword) => evaluate(`{
    const input = document.querySelector('.topbar input');
    input.value = ${JSON.stringify(keyword)};
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
  }`);
  const finishSearch = async (keyword) => {
    await until(() => {
      for (const request of requested(keyword)) {
        if (!request.settled) request.reply();
      }
      return requested(keyword).length === sources.length && requested(keyword).every(request => request.settled);
    }, `${keyword} remaining sources`);
  };
  await rpc("Page.enable");
  await rpc("Page.addScriptToEvaluateOnNewDocument", {
    source: `
      localStorage.setItem('token', 'test');
      window.__errors = [];
      addEventListener('error', event => __errors.push(String(event.message)));
      window.__searchRequests = [];
      window.__searchActive = {};
      window.__searchPeak = {};
      const originalFetch = window.fetch.bind(window);
      // Observe dispatch before HTTP/1.1 connection limits queue the request.
      window.fetch = async (input, init) => {
        const request = new Request(input instanceof Request ? input.clone() : input, init);
        const body = new URL(request.url).pathname === '/graphql' ? await request.json() : null;
        if (body?.operationName !== 'BrowseSource') return originalFetch(input, init);
        const { query: keyword, sourceId: id } = body.variables;
        __searchRequests.push({ keyword, id });
        __searchActive[keyword] = (__searchActive[keyword] || 0) + 1;
        __searchPeak[keyword] = Math.max(__searchPeak[keyword] || 0, __searchActive[keyword]);
        try {
          return await originalFetch(input, init);
        } finally {
          --__searchActive[keyword];
        }
      };
    `,
  });
  await rpc("Page.navigate", { url: `${origin}/catalogue` });
  await until(() => evaluate("document.querySelectorAll('.source-item').length === 16"), "sources");
  await evaluate("document.querySelector('#search').click()");
  await until(() => evaluate("!!document.querySelector('.topbar input')"), "search input");

  await search("parallel");
  await until(async () => (await dispatchedIds("parallel")).length >= 4, "initial parallel requests");
  await until(() => !!sourceRequest("parallel", 4), "initial requests reach server");
  await delay(150);
  assert.deepEqual(await dispatchedIds("parallel"), [1, 2, 3, 4]);
  reply("parallel", 2);
  await until(() => visible("parallel", 2), "healthy source before slow source");
  assert.equal(await visible("parallel", 1), false);
  assert.equal(sourceRequest("parallel", 1).settled, false);
  await until(() => !!sourceRequest("parallel", 5), "freed slot starts next source");
  reply("parallel", 5, true);
  await until(() => !!sourceRequest("parallel", 6), "source failure releases its slot");
  reply("parallel", 6);
  await until(() => visible("parallel", 6), "later healthy source before slow source");
  assert.equal(await visible("parallel", 1), false);
  await finishSearch("parallel");
  await until(async () => (await Promise.all(sources.filter(({ id }) => id !== 5).map(({ id }) => visible("parallel", id)))).every(Boolean), "completed results");
  assert.equal(requested("parallel").length, 16);
  assert.equal(await evaluate("window.__searchPeak.parallel"), 4);

  await search("old");
  await until(async () => (await dispatchedIds("old")).length >= 4, "old search requests");
  await until(() => requested("old").length >= 4, "old requests reach server");
  await search("replacement");
  await until(async () => (await dispatchedIds("replacement")).length >= 4, "replacement search requests");
  await finishSearch("replacement");
  await until(async () => (await Promise.all(sources.map(({ id }) => visible("replacement", id)))).every(Boolean), "replacement results");
  for (const request of requested("old")) request.reply();
  await delay(150);
  assert.deepEqual(await dispatchedIds("old"), [1, 2, 3, 4]);
  assert.equal(await evaluate("window.__searchPeak.replacement"), 4);
  assert.equal(await evaluate("document.body.innerText.includes('old source')"), false);
  const saved = await evaluate("JSON.parse(localStorage.getItem('catalogue_list'))");
  assert.equal(saved.keyword, "replacement");
  assert.deepEqual(Object.values(saved.cover_list_map).map(({ covers }) => covers[0].title).sort(),
    sources.map(({ id }) => `replacement source ${id}`).sort());
  assert.deepEqual(await evaluate("window.__errors"), []);
  await rpc("Page.navigate", { url: "about:blank" });
});
