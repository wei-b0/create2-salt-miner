const DEFAULTS = {
  factory: "0x0000000000FFe8B47B3e2130213B802212439497",
  caller: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
  codehash: "0x64e604787cbf194841e7b68d7cd28786f6c9a0a3ab9f8b0a0e87cb4387ab0107",
  pattern: "010101",
  worksize: 0x4400000,
};

const form = document.getElementById("configForm");
const startBtn = document.getElementById("startBtn");
const stopBtn = document.getElementById("stopBtn");
const log = document.getElementById("log");
const runtimeEl = document.getElementById("runtime");
const rateEl = document.getElementById("rate");
const patternLenEl = document.getElementById("patternLen");
const difficultyEl = document.getElementById("difficulty");
const generatedEl = document.getElementById("generated");
const medianEl = document.getElementById("median");
const progressFill = document.getElementById("progressFill");
const progressLabel = document.getElementById("progressLabel");
const workersValue = document.getElementById("workersValue");
const workersMax = document.getElementById("workersMax");
const hintText = document.getElementById("hintText");

const workerInput = form.elements.workers;
const worksizeInput = form.elements.worksize;

const logicalCores = navigator.hardwareConcurrency || 4;
workerInput.max = logicalCores;
workerInput.value = Math.min(4, logicalCores);
workersMax.textContent = `${logicalCores} cores`;
workersValue.textContent = workerInput.value;

form.elements.factory.value = DEFAULTS.factory;
form.elements.caller.value = DEFAULTS.caller;
form.elements.codehash.value = DEFAULTS.codehash;
form.elements.pattern.value = DEFAULTS.pattern;
worksizeInput.value = `0x${DEFAULTS.worksize.toString(16)}`;

let workers = [];
let stopFlagBuffer = null;
let supportsShared = typeof SharedArrayBuffer !== "undefined";
let running = false;
let startedAt = 0;
let totalAttempts = 0;
let workerRates = [];
let currentPatternBytes = 0;

workerInput.addEventListener("input", () => {
  workersValue.textContent = workerInput.value;
});

function strip0x(value) {
  return value.startsWith("0x") || value.startsWith("0X") ? value.slice(2) : value;
}

function parseWorksize(value) {
  const trimmed = value.trim();
  if (trimmed.startsWith("0x") || trimmed.startsWith("0X")) {
    return Number.parseInt(trimmed, 16);
  }
  return Number.parseInt(trimmed, 10);
}

function buildConfig() {
  const worksize = parseWorksize(worksizeInput.value);
  const pattern = strip0x(form.elements.pattern.value.trim());

  return {
    factory: form.elements.factory.value.trim(),
    caller: form.elements.caller.value.trim(),
    codehash: form.elements.codehash.value.trim(),
    pattern,
    worksize: Number.isFinite(worksize) ? worksize : DEFAULTS.worksize,
  };
}

function setHint(text, isError = false) {
  hintText.textContent = text;
  hintText.style.color = isError ? "#ff6b6b" : "";
}

if (!supportsShared) {
  setHint(
    "SharedArrayBuffer unavailable. Run with COOP/COEP headers for best performance."
  );
}

function appendLog(text) {
  const entry = document.createElement("div");
  entry.className = "log-entry";
  entry.innerHTML = text;
  log.prepend(entry);
}

function updateStatus(rate) {
  if (!running) return;
  const elapsed = (performance.now() - startedAt) / 1000;
  runtimeEl.textContent = `${elapsed.toFixed(1)}s`;
  rateEl.textContent = rate.toFixed(2);

  if (currentPatternBytes > 0) {
    const difficulty = difficultyBigInt(currentPatternBytes);
    const median = medianBigInt(difficulty);
    const prob = probabilityApprox(totalAttempts, difficulty);
    difficultyEl.textContent = formatBigInt(difficulty);
    generatedEl.textContent = formatNumberSpaces(totalAttempts);
    medianEl.textContent = formatBigInt(median);
    const pct = Math.min(100, Math.max(0, prob * 100));
    progressFill.style.width = `${pct.toFixed(2)}%`;
    progressLabel.textContent = `${pct.toFixed(2)}% chance`;
  }
}

function stopMining() {
  running = false;
  startBtn.disabled = false;
  stopBtn.disabled = true;
  if (supportsShared && stopFlagBuffer) {
    const view = new Int32Array(stopFlagBuffer);
    view[0] = 1;
  }
  workers.forEach((worker) => worker.postMessage({ type: "stop" }));
  workers.forEach((worker) => worker.terminate());
  workers = [];
}

function validateConfig(config) {
  const checks = [
    ["factory", config.factory, 40],
    ["caller", config.caller, 40],
    ["codehash", config.codehash, 64],
  ];

  for (const [name, value, length] of checks) {
    const hex = strip0x(value);
    if (hex.length !== length) {
      return `${name} must be ${length} hex chars.`;
    }
  }

  if (!config.pattern || config.pattern.length === 0) {
    return "pattern cannot be empty.";
  }

  if (config.pattern.length / 2 > 20) {
    return "pattern cannot exceed 20 bytes.";
  }

  return null;
}

function formatNumber(value) {
  return Number.isFinite(value) ? value.toLocaleString("en-US") : "0";
}

function formatNumberSpaces(value) {
  return formatNumber(value).replace(/,/g, " ");
}

function formatBigInt(value) {
  const str = value.toString();
  return str.replace(/\B(?=(\d{3})+(?!\d))/g, " ");
}

function difficultyBigInt(patternBytes) {
  return 1n << BigInt(patternBytes * 8);
}

function medianBigInt(difficulty) {
  const numerator = difficulty * 693147n;
  return numerator / 1000000n;
}

function probabilityApprox(attempts, difficulty) {
  if (attempts <= 0) return 0;
  const diffNum = Number(difficulty);
  if (Number.isFinite(diffNum) && diffNum > 0) {
    return 1 - Math.exp(-attempts / diffNum);
  }
  return 0;
}

function startMining() {
  const config = buildConfig();
  const error = validateConfig(config);
  if (error) {
    setHint(error, true);
    return;
  }

  setHint("");
  running = true;
  startBtn.disabled = true;
  stopBtn.disabled = false;
  log.innerHTML = "";

  const workerCount = Math.max(1, Number.parseInt(workerInput.value, 10));
  const batchSize = Math.max(1, Math.floor(config.worksize / workerCount));
  currentPatternBytes = config.pattern.length / 2;
  patternLenEl.textContent = `${currentPatternBytes} bytes`;

  if (config.worksize / workerCount > 4_000_000) {
    setHint("High worksize per worker may freeze UI on slower devices.");
  }

  totalAttempts = 0;
  workerRates = Array.from({ length: workerCount }, () => 0);
  startedAt = performance.now();
  stopFlagBuffer = supportsShared ? new SharedArrayBuffer(4) : new ArrayBuffer(4);
  new Int32Array(stopFlagBuffer)[0] = 0;

  progressFill.style.width = "0%";
  progressLabel.textContent = "0% chance";

  updateStatus(0);

  for (let i = 0; i < workerCount; i += 1) {
    const worker = new Worker(new URL("./worker.js", import.meta.url), {
      type: "module",
    });
    worker.onmessage = (event) => {
      const { type, results, attempts, elapsedMs, message, workerId } =
        event.data;
      if (type === "error") {
        appendLog(`<strong>Worker error:</strong> ${message}`);
        return;
      }

      if (type === "found") {
        results.forEach((result) => {
          appendLog(
            `<strong>${result.address}</strong><br/>Salt: ${result.salt}<br/>Pattern: ${result.pattern}`
          );
        });
        stopMining();
      }

      if (type === "stats") {
        totalAttempts += attempts;
        if (elapsedMs > 0 && typeof workerId === "number") {
          workerRates[workerId] = attempts / (elapsedMs / 1000);
        }
        const avgRate =
          workerRates.reduce((sum, value) => sum + value, 0) / workerRates.length;
        updateStatus(avgRate || 0);
      }
    };

    const seed = (Date.now() + i * 9973) >>> 0;
    worker.postMessage({
      type: "start",
      payload: {
        config,
        seed,
        batchSize,
        workerId: i,
        stopFlagBuffer,
        shared: supportsShared,
      },
    });
    workers.push(worker);
  }
}

startBtn.addEventListener("click", startMining);
stopBtn.addEventListener("click", stopMining);

window.addEventListener("beforeunload", () => {
  stopMining();
});
