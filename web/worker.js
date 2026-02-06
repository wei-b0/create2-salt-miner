import init, { init_worker, run_batch_wasm, set_stop } from "./pkg/salty.js";

let ready = false;
let running = false;
let batchSize = 0;
let workerId = 0;
let lastTick = 0;
let totalAttempts = 0;
let stopFlag = null;

async function ensureReady() {
  if (!ready) {
    await init();
    ready = true;
  }
}

async function tick() {
  if (!running) {
    return;
  }

  if (stopFlag && Atomics.load(stopFlag, 0) === 1) {
    running = false;
    return;
  }

  try {
    const result = run_batch_wasm(batchSize);
    totalAttempts += result.attempts;

    const now = performance.now();
    const elapsedMs = now - lastTick;
    lastTick = now;

    if (result.found && result.found.length > 0) {
      postMessage({ type: "found", results: result.found });
      running = false;
      set_stop(true);
      if (stopFlag) {
        Atomics.store(stopFlag, 0, 1);
      }
      return;
    }

    postMessage({
      type: "stats",
      attempts: result.attempts,
      elapsedMs,
      totalAttempts,
      workerId,
    });
  } catch (error) {
    postMessage({ type: "error", message: String(error) });
    running = false;
  }

  setTimeout(tick, 0);
}

self.onmessage = async (event) => {
  const { type, payload } = event.data;

  if (type === "start") {
    await ensureReady();
    running = true;
    batchSize = payload.batchSize;
    workerId = payload.workerId;
    stopFlag =
      payload.shared && payload.stopFlagBuffer
        ? new Int32Array(payload.stopFlagBuffer)
        : null;
    totalAttempts = 0;
    lastTick = performance.now();
    try {
      init_worker(payload.config, payload.seed, workerId);
      set_stop(false);
      tick();
    } catch (error) {
      postMessage({ type: "error", message: String(error) });
      running = false;
    }
  }

  if (type === "stop") {
    running = false;
    set_stop(true);
    if (stopFlag) {
      Atomics.store(stopFlag, 0, 1);
    }
  }
};
