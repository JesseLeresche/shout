const { listen } = window.__TAURI__.event;

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
