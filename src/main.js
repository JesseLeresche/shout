const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const stateEl = document.getElementById("state");
const dotEl = document.getElementById("dot");
const detailEl = document.getElementById("detail");
const lastEl = document.getElementById("last");
const rawEl = document.getElementById("raw");
const cleanedEl = document.getElementById("cleaned");

listen("shout:status", ({ payload }) => {
  stateEl.textContent = payload.state;
  dotEl.className = payload.state;
  detailEl.textContent = payload.detail ?? "";
});

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
  "ghost_hotkey",
  "vault_dir",
  "ghost_input_device",
];
const field = (name) => document.getElementById(`f-${name}`);

async function loadSettings() {
  const cfg = await invoke("get_config");
  for (const name of FIELDS) field(name).value = cfg[name] ?? "";
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
  const note = document.getElementById("save-note");
  try {
    await invoke("save_config", { cfg });
    currentCfg = cfg;
    note.textContent = "saved — restart shout to apply hotkey changes";
  } catch (e) {
    note.textContent = `save failed: ${e}`;
  }
});
