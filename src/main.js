const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const stateEl = document.getElementById("state");
const dotEl = document.getElementById("dot");
const detailEl = document.getElementById("detail");
const lastEl = document.getElementById("last");
const rawEl = document.getElementById("raw");
const cleanedEl = document.getElementById("cleaned");

function showStatus(payload) {
  stateEl.textContent = payload.state;
  dotEl.className = payload.state;
  detailEl.textContent = payload.detail ?? "";
}

listen("shout:status", ({ payload }) => showStatus(payload));

// Catch up on the status emitted before this webview attached its listener.
invoke("get_status").then(showStatus);

listen("shout:result", ({ payload }) => {
  lastEl.hidden = false;
  rawEl.textContent = payload.raw;
  cleanedEl.textContent = payload.cleaned;
});

// --- Settings ---
const FIELDS = [
  "ollama_url",
  "ollama_model",
  "ollama_summary_model",
  "hotkey",
  "input_device",
  "ghost_hotkey",
  "vault_dir",
  "ghost_input_device",
  "parakeet_model_dir",
  "whisper_model",
];
const field = (name) => document.getElementById(`f-${name}`);

async function loadSettings() {
  const cfg = await invoke("get_config");
  for (const name of FIELDS) field(name).value = cfg[name] ?? "";
  field("live_typing").checked = !!cfg.live_typing;
  document.getElementById("hk-dictate").textContent = cfg.hotkey;
  document.getElementById("hk-ghost").textContent = cfg.ghost_hotkey;
  return cfg;
}

let currentCfg = null;
loadSettings().then((cfg) => (currentCfg = cfg));

document.getElementById("save").addEventListener("click", async () => {
  const cfg = { ...(currentCfg ?? {}) };
  for (const name of FIELDS) {
    const v = field(name).value.trim();
    cfg[name] = v === "" ? null : v;
  }
  // required string fields must not be null
  for (const req of ["ollama_url", "ollama_model", "ollama_summary_model", "hotkey", "ghost_hotkey"]) {
    if (!cfg[req]) cfg[req] = currentCfg?.[req];
  }
  cfg.live_typing = field("live_typing").checked;
  const note = document.getElementById("save-note");
  try {
    await invoke("save_config", { cfg });
    currentCfg = cfg;
    note.textContent =
      "saved — streaming mode applies now; hotkey/mic changes need a restart";
  } catch (e) {
    note.textContent = `save failed: ${e}`;
  }
});

// --- Models ---
const modelsListEl = document.getElementById("models-list");

function formatBytes(n) {
  if (n == null) return "";
  const mb = n / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)}GB` : `${mb.toFixed(0)}MB`;
}

function modelRowHtml(m) {
  return `
    <div class="model-row" id="model-${m.id}">
      <div class="model-info">
        <span class="model-label">${m.label}</span>
        <span class="model-meta">${m.category} · ~${m.approx_mb}MB</span>
      </div>
      <div class="model-status">
        <span class="model-status-text">${m.installed ? "installed" : "not installed"}</span>
        ${m.installed ? "" : `<button class="dl" data-id="${m.id}">Download</button>`}
        <progress class="dl-progress" max="100" value="0" hidden></progress>
      </div>
    </div>`;
}

async function loadModels() {
  const models = await invoke("list_models");
  modelsListEl.innerHTML = models.map(modelRowHtml).join("");
  for (const btn of modelsListEl.querySelectorAll(".dl")) {
    btn.addEventListener("click", () => startDownload(btn.dataset.id));
  }
}

function startDownload(id) {
  const row = document.getElementById(`model-${id}`);
  const btn = row.querySelector(".dl");
  const progress = row.querySelector(".dl-progress");
  const statusText = row.querySelector(".model-status-text");
  btn.disabled = true;
  progress.hidden = false;
  statusText.textContent = "starting…";
  invoke("download_model", { id }).catch((e) => {
    statusText.textContent = `failed: ${e}`;
    btn.disabled = false;
  });
}

listen("shout:model-progress", ({ payload }) => {
  const row = document.getElementById(`model-${payload.id}`);
  if (!row) return;
  const btn = row.querySelector(".dl");
  const progress = row.querySelector(".dl-progress");
  const statusText = row.querySelector(".model-status-text");
  if (payload.phase === "downloading") {
    statusText.textContent = payload.total
      ? `downloading ${formatBytes(payload.downloaded)} / ${formatBytes(payload.total)}`
      : `downloading ${formatBytes(payload.downloaded)}`;
    if (payload.total) {
      progress.max = payload.total;
      progress.value = payload.downloaded;
    }
  } else if (payload.phase === "extracting") {
    statusText.textContent = "extracting…";
  } else if (payload.phase === "done") {
    loadModels();
  } else if (payload.phase === "error") {
    statusText.textContent = `failed: ${payload.message}`;
    if (btn) btn.disabled = false;
    progress.hidden = true;
  }
});

loadModels();
