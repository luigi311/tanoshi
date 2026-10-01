// Paste this entire file into the developer console of the running app.
//
// Visual check (simulated latency, no queue changes):
//   tanoshiQueueRepro.slowResponses(3000)
//   Navigate using the app's Library, Catalogue, and Settings tabs.
//   tanoshiQueueRepro.restore()
//
// Actual queue load using only dummy entries (requires the updated backend):
//   Pause downloads in Settings > Download Queue first.
//   await tanoshiQueueRepro.seed(5000, 30)
//   tanoshiQueueRepro.cancelFront(100) // five parallel requests for a longer test
//   Use cancelFront(5) instead for the automated regression's smaller workload.
//   Navigate while the returned promise is pending, then:
//   await tanoshiQueueRepro.cleanup()
//   Resume downloads when finished. Dummy URLs fail if downloads are resumed
//   before cleanup. No actual manga, chapter, or library records are seeded.
//
// Re-evaluating this file restores the previous helper first. Reloading the
// page removes the response-delay wrapper. In-flight server mutations continue
// independently; restoring fetch does not undo or stop chapter cancellations.
(() => {
  const session = window.tanoshiQueueRepro?._session ?? { busy: false };
  window.tanoshiQueueRepro?.restore();
  const originalFetch = window.fetch;
  const pendingDelays = new Set();
  const storageKey = "tanoshi-queue-repro-run";
  let delayedFetch;

  function restore() {
    if (window.fetch === delayedFetch) window.fetch = originalFetch;
    delayedFetch = undefined;
    for (const pending of pendingDelays) {
      window.clearTimeout(pending.timer);
      pending.resolve();
    }
    pendingDelays.clear();
  }

  function slowResponses(milliseconds = 3000) {
    if (!Number.isFinite(milliseconds) || milliseconds < 0) {
      throw new Error("Delay must be a non-negative number of milliseconds.");
    }
    restore();
    const wrapper = async function (input, options) {
      const response = await originalFetch.call(window, input, options);
      const url = new URL(input?.url ?? input, window.location.href);
      if (window.fetch === wrapper && /^\/graphql\/?$/.test(url.pathname)) {
        await new Promise((resolve) => {
          const pending = { resolve };
          pending.timer = window.setTimeout(() => {
            pendingDelays.delete(pending);
            resolve();
          }, milliseconds);
          pendingDelays.add(pending);
        });
      }
      return response;
    };
    delayedFetch = wrapper;
    window.fetch = wrapper;
    console.info(
      `Simulating ${milliseconds} ms of GraphQL response delay. ` +
      "Navigate within the app; run tanoshiQueueRepro.restore() to stop."
    );
  }

  async function graphql(query, variables = {}) {
    const token = window.localStorage.getItem("token");
    if (!token) throw new Error("Sign in as an admin before testing the queue.");
    const port = window.__TANOSHI_PORT__;
    const endpoint = Number.isInteger(port) && port > 0 && port <= 65535
      ? `http://localhost:${port}/graphql`
      : new URL("/graphql", window.location.href).href;
    // Use the original fetch so simulated latency does not affect this test.
    const response = await originalFetch.call(window, endpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({ query, variables }),
    });
    if (!response.ok) throw new Error(`GraphQL HTTP ${response.status}`);
    const result = await response.json();
    if (result.errors?.length) {
      throw new Error(result.errors.map((error) => error.message).join("; "));
    }
    if (!result.data) throw new Error("GraphQL response has no data.");
    return result.data;
  }

  function currentRun() {
    const stored = window.localStorage.getItem(storageKey);
    if (!stored) throw new Error("Run tanoshiQueueRepro.seed() before cancelling or cleaning up dummy chapters.");
    const run = JSON.parse(stored);
    if (!/^[a-f0-9]{32}$/.test(run.runId)) throw new Error("Invalid stored queue test run.");
    return run;
  }

  async function exclusive(action) {
    if (session.busy) throw new Error("Wait for the current queue test operation to finish.");
    session.busy = true;
    try {
      restore();
      return await action();
    } finally {
      session.busy = false;
    }
  }

  async function seed(chapters = 5000, pagesPerChapter = 30) {
    if (!Number.isInteger(chapters) || chapters < 1 || chapters > 5000) {
      throw new Error("Chapter count must be between 1 and 5000.");
    }
    if (!Number.isInteger(pagesPerChapter) || pagesPerChapter < 1 || pagesPerChapter > 100) {
      throw new Error("Page count must be between 1 and 100.");
    }
    return exclusive(async () => {
      const { downloadStatus } = await graphql("{ downloadStatus }");
      if (downloadStatus) throw new Error("Pause downloads in Settings > Download Queue before seeding dummy chapters.");
      let run;
      if (window.localStorage.getItem(storageKey)) {
        run = currentRun();
        if (run.chapters !== chapters || run.pagesPerChapter !== pagesPerChapter) {
          throw new Error("Run cleanup() before seeding a different queue size.");
        }
      } else {
        const bytes = window.crypto.getRandomValues(new Uint8Array(16));
        const runId = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
        run = { runId, chapters, pagesPerChapter };
        // Save before sending so cleanup still works if the response is lost.
        window.localStorage.setItem(storageKey, JSON.stringify(run));
      }
      const data = await graphql(
        "mutation SeedQueueRepro($runId: String!, $chapters: Int!, $pages: Int!) { " +
        "seedDownloadQueueRepro(runId: $runId, chapters: $chapters, pagesPerChapter: $pages) }",
        { runId: run.runId, chapters, pages: pagesPerChapter }
      );
      const ids = data.seedDownloadQueueRepro;
      console.info(
        `Queue test ready: ${ids.length} dummy chapters, ${pagesPerChapter} pages each. ` +
        "Run cancelFront(count) to test, then cleanup() to remove the remaining dummy entries."
      );
      return ids;
    });
  }

  async function cancelFront(count = 5) {
    if (!Number.isInteger(count) || count < 1) {
      throw new Error("Chapter count must be a positive integer.");
    }
    return exclusive(async () => {
      const run = currentRun();
      const { downloadStatus, downloadQueue } = await graphql(
        "{ downloadStatus downloadQueue { chapterId sourceId sourceName mangaTitle } }"
      );
      if (downloadStatus) {
        throw new Error("Pause downloads in Settings > Download Queue before running this test.");
      }
      const dummyChapters = downloadQueue.filter((chapter) =>
        chapter.sourceId === -1 && chapter.sourceName === "Tanoshi queue repro" &&
        chapter.mangaTitle === `Queue repro - ${run.runId}` &&
        Number.isSafeInteger(chapter.chapterId) && chapter.chapterId < 0
      );
      if (dummyChapters.length < count) {
        throw new Error(`This test run contains ${dummyChapters.length} dummy chapters; requested ${count}.`);
      }
      const selected = dummyChapters.slice(0, count);
      const groups = Array.from({ length: Math.min(5, count) }, () => []);
      selected.forEach((chapter, index) => {
        groups[index % groups.length].push(chapter.chapterId);
      });
      console.info(
        `Cancelling the first ${count} dummy chapters from this ${dummyChapters.length}-chapter ` +
        `test run in ${groups.length} parallel requests. Navigate now to compare the delays.`
      );
      const results = await Promise.allSettled(groups.map(async (ids) => {
        const started = performance.now();
        try {
          const data = await graphql(
            "mutation QueueRepro($ids: [Int!]!) { removeChaptersFromQueue(ids: $ids) }",
            { ids }
          );
          return {
            requested: ids.length,
            removed: data.removeChaptersFromQueue,
            milliseconds: Math.round(performance.now() - started),
          };
        } catch (error) {
          console.error("Cancellation failed; some chapters may already have been removed.", error);
          throw error;
        }
      }));
      console.table(results.map((result, index) => result.status === "fulfilled"
        ? { request: index + 1, ...result.value }
        : { request: index + 1, error: result.reason.message }));
      return results;
    });
  }

  async function cleanup() {
    return exclusive(async () => {
      const run = currentRun();
      const data = await graphql(
        "mutation ClearQueueRepro($runId: String!) { clearDownloadQueueRepro(runId: $runId) }",
        { runId: run.runId }
      );
      window.localStorage.removeItem(storageKey);
      console.info(`Removed ${data.clearDownloadQueueRepro} remaining dummy chapters. Downloads can be resumed.`);
      return data.clearDownloadQueueRepro;
    });
  }

  window.tanoshiQueueRepro = { seed, cancelFront, cleanup, slowResponses, restore, _session: session };
  console.info(
    "Queue reproduction helper ready: seed(chapters, pages), cancelFront(count), cleanup(), slowResponses(ms), restore()."
  );
})();
