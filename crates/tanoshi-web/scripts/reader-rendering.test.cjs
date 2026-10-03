// Build first: cd crates/tanoshi-web && trunk build --release
// Run: node --test crates/tanoshi-web/scripts/reader-rendering.test.cjs
// Optional: TANOSHI_WEB_DIST=/path/to/dist CHROMIUM=/path/to/chromium
const assert = require("node:assert/strict");
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const { test } = require("node:test");

const dist = path.resolve(process.env.TANOSHI_WEB_DIST || path.join(__dirname, "../dist"));
const pageCount = 400;
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const image = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aN1sAAAAASUVORK5CYII=", "base64");
const shortPanel = '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="40"><rect width="400" height="40" fill="red"/></svg>';
const doubleCases = new Map([
  ["double portrait before landscape", { dimensions: [[800, 400], [400, 800]], order: [1, 0], paired: false }],
  ["double landscape before portrait", { dimensions: [[800, 400], [400, 800]], order: [0, 1], paired: false }],
  ["double portraits second first", { dimensions: [[400, 800], [400, 800]], order: [1, 0], paired: true }],
  ["double portraits first first", { dimensions: [[400, 800], [400, 800]], order: [0, 1], paired: true }],
  ["double late previous landscape", { dimensions: [null, [800, 400], [400, 800]], order: [2, 1], startPage: 3, latePrevious: true }],
  ["single to double portraits", { dimensions: [[400, 800], [400, 800]], order: [0, 1], paired: true, switchMode: true }],
  ["single to double landscape", { dimensions: [[800, 400], [400, 800]], order: [0, 1], paired: false, switchMode: true }],
  ["continuous to double portraits", { dimensions: [[400, 800], [400, 800]], order: [0, 1], paired: true, switchMode: true }],
]);

test("image updates preserve unrelated reader pages", { timeout: 60000 }, async (t) => {
  assert.ok(fs.existsSync(path.join(dist, "index.html")), "build tanoshi-web before running this test");
  const pendingImages = new Map();
  let shortPanels = false;
  const server = http.createServer((request, response) => {
    const pathname = new URL(request.url, "http://fixture").pathname;
    // Hold images so load/error events can be tested independently.
    if (pathname.startsWith("/image/")) {
      if (shortPanels) {
        response.writeHead(200, { "Content-Type": "image/svg+xml", "Cache-Control": "no-store" });
        response.end(shortPanel);
        return;
      }
      pendingImages.set(pathname, response);
      return;
    }
    if (pathname === "/sw.js") {
      response.writeHead(200, { "Content-Type": "text/javascript" });
      response.end("");
      return;
    }
    if (pathname === "/graphql") {
      let body = "";
      request.on("data", (chunk) => { body += chunk; });
      request.on("end", () => {
        const { operationName } = JSON.parse(body);
        const results = {
          FetchServerStatus: { serverStatus: { activated: true, loggedin: true, version: "test" } },
          FetchChapter: { chapter: {
            title: "Reader fixture", number: 1, prev: null, next: null,
            source: { id: 1, url: "https://fixture.invalid" },
            manga: { id: 1, title: "Reader fixture" },
            pages: Array.from({ length: pageCount }, (_, index) => `page-${index}.png`),
          } },
          UpdatePageReadAt: { updatePageReadAt: 1 },
        };
        response.writeHead(200, { "Content-Type": "application/json" });
        response.end(JSON.stringify(operationName in results
          ? { data: results[operationName] }
          : { errors: [{ message: `Unexpected operation: ${operationName}` }] }));
      });
      return;
    }
    let file = path.join(dist, pathname);
    if (!fs.existsSync(file) || !fs.statSync(file).isFile()) file = path.join(dist, "index.html");
    const mime = { ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css", ".html": "text/html", ".png": "image/png" };
    response.writeHead(200, { "Content-Type": mime[path.extname(file)] || "application/octet-stream" });
    fs.createReadStream(file).pipe(response);
  });
  t.after(() => { server.closeAllConnections(); server.close(); });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const origin = `http://127.0.0.1:${server.address().port}`;
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), "tanoshi-reader-test-"));
  const browser = spawn(process.env.CHROMIUM || "chromium", [
    "--headless", "--disable-gpu", "--no-first-run", "--remote-debugging-pipe", `--user-data-dir=${profile}`,
  ], { stdio: ["ignore", "ignore", "pipe", "pipe", "pipe"] });
  t.after(async () => {
    if (browser.pid && browser.exitCode === null && browser.signalCode === null) {
      const exited = new Promise((resolve) => browser.once("exit", resolve));
      browser.kill();
      await exited;
    }
    fs.rmSync(profile, { recursive: true, force: true });
  });

  let sequence = 0, buffer = "", stderr = "";
  const calls = new Map();
  const rejectCalls = (error) => {
    for (const call of calls.values()) call.reject(error);
    calls.clear();
  };
  browser.on("error", rejectCalls);
  browser.on("exit", () => rejectCalls(new Error(`Chromium exited: ${stderr.slice(-1000)}`)));
  browser.stderr.on("data", (chunk) => { stderr += chunk; });
  browser.stdio[4].on("data", (chunk) => {
    buffer += chunk;
    let end;
    while ((end = buffer.indexOf("\0")) >= 0) {
      const message = JSON.parse(buffer.slice(0, end));
      buffer = buffer.slice(end + 1);
      const call = calls.get(message.id);
      if (call) {
        calls.delete(message.id);
        message.error ? call.reject(new Error(JSON.stringify(message.error))) : call.resolve(message.result);
      }
    }
  });
  const call = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const id = ++sequence;
    calls.set(id, { resolve, reject });
    browser.stdio[3].write(JSON.stringify({ id, method, params, sessionId }) + "\0");
  });

  for (const mode of ["continuous scroll", "continuous scroll narrow", "continuous resume", "continuous", "single", "double", ...doubleCases.keys()]) {
    await t.test(mode, async (t) => {
      pendingImages.clear();
      shortPanels = false;
      const { browserContextId } = await call("Target.createBrowserContext");
      t.after(() => call("Target.disposeBrowserContext", { browserContextId }));
      const { targetId } = await call("Target.createTarget", { url: "about:blank", browserContextId });
      const { sessionId } = await call("Target.attachToTarget", { targetId, flatten: true });
      const rpc = (method, params) => call(method, params, sessionId);
      const evaluate = async (expression) => {
        const result = await rpc("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
        if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
        return result.result.value;
      };
      const until = async (expression) => {
        for (let attempt = 0; attempt < 100; attempt++) {
          if (await evaluate(expression)) return;
          await delay(50);
        }
        assert.fail(`Timed out waiting for ${expression}`);
      };
      await rpc("Page.enable");
      if (mode.endsWith("narrow")) {
        await rpc("Emulation.setDeviceMetricsOverride", { width: 360, height: 800, deviceScaleFactor: 1, mobile: false });
      }
      await rpc("Page.addScriptToEvaluateOnNewDocument", { source: `
        localStorage.setItem("token", "test");
        localStorage.setItem("settings:reader", JSON.stringify({
          reader_mode: "${mode.startsWith("continuous") ? "Continous" : "Paged"}",
          display_mode: "${mode.startsWith("double") ? "Double" : "Single"}",
          padding: false, direction: "LeftToRight", background: "White", fit: "All"
        }));
      ` });
      const startPage = doubleCases.get(mode)?.startPage || (mode.endsWith("resume") ? 101 : 1);
      await rpc("Page.navigate", { url: `${origin}/chapter/1#${startPage}` });
      await until(`document.querySelectorAll("#page-list img").length === ${pageCount}`);
      await evaluate(`
        window.originalPages = [...document.querySelectorAll("#page-list img")];
        window.changes = { added: 0, removed: 0 };
        new MutationObserver(records => {
          for (const record of records) {
            changes.added += [...record.addedNodes].filter(node => node.tagName === "IMG").length;
            changes.removed += [...record.removedNodes].filter(node => node.tagName === "IMG").length;
          }
        }).observe(document.querySelector("#page-list"), { childList: true });
      `);

      if (doubleCases.has(mode)) {
        const { dimensions, order, paired, latePrevious, switchMode } = doubleCases.get(mode);
        for (const index of order) {
          const pathname = `/image/page-${index}.png`;
          for (let attempt = 0; !pendingImages.has(pathname) && attempt < 100; attempt++) await delay(50);
          const response = pendingImages.get(pathname);
          assert.ok(response, `page ${index + 1} was requested`);
          const [width, height] = dimensions[index];
          response.writeHead(200, { "Content-Type": "image/svg+xml", "Cache-Control": "max-age=3600" });
          response.end(`<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><rect width="${width}" height="${height}" fill="red"/></svg>`);
          await until(`document.querySelectorAll("#page-list img")[${index}].naturalWidth === ${width} && document.querySelectorAll("#page-list img")[${index}].naturalHeight === ${height}`);
          await delay(50);
          if (index === 1) await evaluate('window.secondPage = document.getElementById("1")');
          if (index === 2) await evaluate('window.thirdPage = document.getElementById("2")');
        }
        if (switchMode) {
          // Delay the new nodes' load handlers so the initial spread must use existing dimensions.
          await evaluate(`
            window.oldPageList = document.getElementById("page-list");
            window.blockedLoads = 0;
            document.addEventListener("load", event => {
              if (event.target.matches?.("#page-list img")) {
                blockedLoads++;
                event.stopImmediatePropagation();
              }
            }, true);
            const buttons = [...document.querySelectorAll(".reader-settings button")];
            buttons.find(button => button.textContent === "Paged").click();
            buttons.find(button => button.textContent === "Double").click();
          `);
          await until('document.getElementById("page-list") !== oldPageList && blockedLoads >= 2');
          assert.deepEqual(await evaluate('[...document.querySelectorAll("#page-list img")].filter(img => getComputedStyle(img).display !== "none").map(img => img.id)'), paired ? ["0", "1"] : ["0"],
            "the spread must be correct before new image load handlers run");
          assert.equal(await evaluate('document.getElementById("0").style.width'), paired ? "50%" : "initial");
          await evaluate('document.getElementById("next").click()');
          await until(`location.hash === "#${paired ? 3 : 2}"`);
          return;
        }
        if (latePrevious) {
          assert.equal(await evaluate('document.getElementById("2") === thirdPage'), true);
          await evaluate('document.getElementById("prev").click()');
          await until('location.hash === "#2"');
          assert.deepEqual(await evaluate('[...document.querySelectorAll("#page-list img")].filter(img => getComputedStyle(img).display !== "none").map(img => img.id)'), ["1"]);
          await evaluate('document.getElementById("next").click()');
          await until('location.hash === "#3"');
          assert.deepEqual(await evaluate("changes"), { added: 2, removed: 2 });
          return;
        }
        const spread = await evaluate(`({
          visible: [...document.querySelectorAll("#page-list img")].filter(img => getComputedStyle(img).display !== "none").map(img => img.id),
          width: document.getElementById("0").style.width,
          preservedSecond: document.getElementById("1") === secondPage,
          preservedOther: originalPages.slice(2).every(img => img.isConnected)
        })`);
        assert.deepEqual(spread.visible, paired ? ["0", "1"] : ["0"], "visibility must follow both pages' dimensions, regardless of load order");
        assert.equal(spread.width, paired ? "50%" : "initial");
        assert.equal(spread.preservedSecond, true, "loading the first page must retain its neighbour");
        assert.equal(spread.preservedOther, true, "unrelated reader pages must retain their DOM");
        assert.deepEqual(await evaluate("changes"), { added: 2, removed: 2 });
        await evaluate('document.getElementById("next").click()');
        await until(`location.hash === "#${paired ? 3 : 2}"`);
        await evaluate('document.getElementById("prev").click()');
        await until('location.hash === "#1"');
        assert.deepEqual(await evaluate("changes"), { added: 2, removed: 2 }, "navigation must not replace images");
        return;
      }

      if (mode.startsWith("continuous scroll") || mode.endsWith("resume")) {
        // Jump far down while requests are pending, then stop scrolling as panels load.
        if (!mode.endsWith("resume")) {
          await evaluate('document.getElementById("100").scrollIntoView()');
          await until('location.hash === "#101"');
        } else {
          for (let attempt = 0; !pendingImages.has("/image/page-100.png") && attempt < 100; attempt++) await delay(50);
          assert.ok(pendingImages.has("/image/page-100.png"), "request the bookmarked page before restoring its scroll position");
          await delay(200);
          assert.equal(await evaluate("location.hash"), "#101", "a delayed image must not reset the bookmarked page");
        }
        shortPanels = true;
        for (const response of pendingImages.values()) {
          if (!response.destroyed) {
            response.writeHead(200, { "Content-Type": "image/svg+xml", "Cache-Control": "no-store" });
            response.end(shortPanel);
          }
        }
        await delay(1000);
        assert.ok(await evaluate("scrollY > 10000"), "keep the reader far down the chapter");
        const visible = await evaluate(`
          [...document.querySelectorAll("#page-list img")].filter(img => {
            const rect = img.getBoundingClientRect();
            return rect.bottom > 0 && rect.top < innerHeight;
          }).map(img => ({ id: img.id, src: img.getAttribute("src"), complete: img.complete,
            height: img.naturalHeight, loading: img.classList.contains("continuous-image-loading") }))
        `);
        assert.ok(visible.length > 0);
        assert.ok(visible.every(img => img.src && img.complete && img.height > 0 && !img.loading),
          `visible panels must load without another scroll: ${JSON.stringify(visible)}`);
        return;
      }

      // Complete an actual request, including the cached reload of its replacement.
      for (let attempt = 0; !pendingImages.has("/image/page-0.png") && attempt < 100; attempt++) await delay(50);
      const first = pendingImages.get("/image/page-0.png");
      assert.ok(first, "the first page was requested");
      first.writeHead(200, { "Content-Type": "image/png", "Cache-Control": "max-age=3600" });
      first.end(image);
      await until("changes.added > 0");
      await delay(100);
      assert.deepEqual(await evaluate("changes"), { added: 1, removed: 1 });
      assert.equal(await evaluate("originalPages.filter(node => node.isConnected).length"), pageCount - 1);
      if (mode === "continuous") {
        assert.equal(await evaluate('document.getElementById("0").classList.contains("continuous-image-loading")'), false);
      }

      // Separate updates must stay linear in the number of completed pages.
      for (let index = 1; index < 10; index++) {
        await evaluate(`originalPages[${index}].dispatchEvent(new Event("load"))`);
        await until(`changes.added >= ${index + 1}`);
      }
      assert.deepEqual(await evaluate("changes"), { added: 10, removed: 10 });
      assert.equal(await evaluate("originalPages.filter(node => node.isConnected).length"), pageCount - 10);

      // Error and retry replace just that page, keeping its slot and neighbours.
      await evaluate('originalPages[10].dispatchEvent(new Event("error"))');
      await until('document.getElementById("10")?.querySelector("button")');
      assert.equal(await evaluate("changes.removed"), 11);
      assert.equal(await evaluate("originalPages.slice(11).every(node => node.isConnected)"), true);
      await evaluate('document.getElementById("10").querySelector("button").click()');
      await until(`document.querySelectorAll("#page-list img").length === ${pageCount}`);
      assert.deepEqual(await evaluate("changes"), { added: 11, removed: 11 });
      assert.equal(await evaluate('document.querySelectorAll("#page-list img")[10].getAttribute("src")'), null);
    });
  }
});
