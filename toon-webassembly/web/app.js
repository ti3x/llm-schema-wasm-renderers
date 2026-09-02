// Bidirectional JSON ⇄ TOON playground.
//
// Whichever pane the user last typed in is the source of truth; the
// other is regenerated. A `silent` flag prevents the WASM-emitted
// output from re-triggering a conversion in the opposite direction.
import init, { json_to_toon, toon_to_json } from "./pkg/toon_wasm.js";

const $ = (id) => document.getElementById(id);

const jsonEl = $("json-pane");
const toonEl = $("toon-pane");
const errEl = $("err");
const sizeEl = $("size");

const jsonCharsEl = $("json-chars");
const jsonTokensEl = $("json-tokens");
const toonCharsEl = $("toon-chars");
const toonTokensEl = $("toon-tokens");
const savingsEl = $("savings");

const indentEl = $("indent");
const strictEl = $("strict");
const coerceEl = $("coerce");
const expandEl = $("expand-paths");
const foldEl = $("key-folding");

const fmt = new Intl.NumberFormat();

function approxTokens(s) {
  return Math.ceil(s.length / 4);
}

function updateStats() {
  const j = jsonEl.value.length;
  const t = toonEl.value.length;
  const jt = approxTokens(jsonEl.value);
  const tt = approxTokens(toonEl.value);
  jsonCharsEl.textContent = fmt.format(j);
  toonCharsEl.textContent = fmt.format(t);
  jsonTokensEl.textContent = fmt.format(jt);
  toonTokensEl.textContent = fmt.format(tt);
  // "% saved" = how much shorter TOON is than the source JSON.
  // Positive when TOON < JSON (the common case).
  const pct = jt > 0 ? ((jt - tt) / jt) * 100 : 0;
  savingsEl.textContent = `${pct.toFixed(1)}%`;
  savingsEl.parentElement.dataset.sign = pct > 0 ? "pos" : pct < 0 ? "neg" : "zero";
}

function getDelimiter() {
  for (const r of document.querySelectorAll('input[name="delimiter"]')) {
    if (r.checked) return r.value;
  }
  return "comma";
}

function getOptions() {
  return JSON.stringify({
    delimiter: getDelimiter(),
    indent: Number(indentEl.value) || 2,
    strict: strictEl.checked,
    coerceTypes: coerceEl.checked,
    keyFolding: foldEl.checked,
    expandPaths: expandEl.checked,
  });
}

function showError(msg) {
  errEl.hidden = false;
  errEl.textContent = msg;
}
function clearError() {
  errEl.hidden = true;
  errEl.textContent = "";
}

let silent = false;
let debounceId;

function scheduleFrom(direction) {
  clearTimeout(debounceId);
  debounceId = setTimeout(() => convert(direction), 120);
}

function convert(direction) {
  if (silent) return;
  const opts = getOptions();
  try {
    silent = true;
    if (direction === "json") {
      const out = json_to_toon(jsonEl.value, opts);
      toonEl.value = out;
    } else {
      const out = toon_to_json(toonEl.value, opts);
      jsonEl.value = out;
    }
    clearError();
  } catch (e) {
    showError(String(e?.message ?? e));
  } finally {
    silent = false;
    updateStats();
  }
}

// Re-run conversion when any setting changes — always from the JSON pane
// (the canonical source for option-driven re-encoding).
function settingsChanged() {
  // If the TOON pane is empty, prefer json→toon; if JSON pane is empty
  // prefer toon→json. Otherwise default to json→toon.
  if (!toonEl.value.trim() && jsonEl.value.trim()) {
    convert("json");
  } else if (!jsonEl.value.trim() && toonEl.value.trim()) {
    convert("toon");
  } else {
    convert("json");
  }
}

async function main() {
  await init();
  try {
    const res = await fetch("./pkg/toon_wasm_bg.wasm", { method: "HEAD" });
    const sz = res.headers.get("content-length");
    if (sz) sizeEl.textContent = `· WASM ${(sz / 1024).toFixed(1)} KB`;
  } catch (_) { /* ignore */ }

  jsonEl.addEventListener("input", () => scheduleFrom("json"));
  toonEl.addEventListener("input", () => scheduleFrom("toon"));

  for (const r of document.querySelectorAll('input[name="delimiter"]')) {
    r.addEventListener("change", settingsChanged);
  }
  for (const el of [indentEl, strictEl, coerceEl, expandEl, foldEl]) {
    el.addEventListener("change", settingsChanged);
  }

  convert("json");
}

main().catch((e) => showError("Failed to initialize WASM: " + e));
