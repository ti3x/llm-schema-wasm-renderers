// toon → html playground. One WebAssembly module does the whole pipeline:
// TOON → JSON UI spec → (adapt json-render.dev form) → HTML.
import init, { compile } from "./pkg/toon_render_wasm.js";

const $ = (id) => document.getElementById(id);
const toonEl = $("toon");
const htmlEl = $("html");
const preview = $("preview");
const errEl = $("err");
const presetEl = $("preset");
const statToon = $("stat-toon");
const statHtml = $("stat-html");

const fmt = new Intl.NumberFormat();
const approxTokens = (s) => Math.ceil(s.length / 4);

// TOON presets (generated from JSON via toon-wasm, so they decode cleanly).
const PRESETS = {
  dev: `root: card
state:
  user: Ada Lovelace
  visits: 128
  trend[7]: 3,7,4,9,12,8,15
elements:
  card:
    type: Card
    props:
      title: User dashboard
    children[3]: hi,m,g
  hi:
    type: Heading
    props:
      level: 3
      text:
        "$template": "Welcome, \${/user}"
  m:
    type: Metric
    props:
      label: Visits
      value:
        "$state": /visits
      change: +8%
  g:
    type: LineGraph
    props:
      data:
        "$state": /trend
`,
  flat: `tag: Card
props:
  title: Plain card
children[3]:
  - tag: Heading
    props:
      level: 4
      value: A flat spec
  - tag: Text
    props:
      value: "No indirection, no bindings — rendered as-is."
  - tag: Button
    props:
      label: Click me
      variant: primary
`,
};

// Minimal styling for the sandboxed preview so the markup is legible.
const PREVIEW_CSS = `
  body { font: 14px/1.5 system-ui, sans-serif; margin: 12px; color: #1a1a1a; }
  .jr-card { border: 1px solid #d5d5db; border-radius: 10px; overflow: hidden; }
  .jr-card-title { background: #f4f4f6; padding: 8px 12px; font-weight: 600; }
  .jr-card-body { padding: 12px; display: grid; gap: 8px; }
  .jr-container { display: grid; gap: 6px; }
  .jr-heading { margin: 0; }
  .jr-text { color: #333; }
  .jr-btn { padding: 6px 12px; border-radius: 6px; border: 1px solid #c7c7cf;
            background: #fff; cursor: pointer; justify-self: start; }
  .jr-btn-primary { background: #2563eb; color: #fff; border-color: #2563eb; }
`;

function frame(html) {
  return `<!doctype html><html><head><meta charset="utf-8">
    <style>${PREVIEW_CSS}</style></head><body>${html}</body></html>`;
}

function run() {
  const toon = toonEl.value;
  statToon.textContent = `${fmt.format(toon.length)} chars · ~${fmt.format(approxTokens(toon))} tokens`;
  try {
    const html = compile(toon, "");
    errEl.hidden = true;
    htmlEl.value = html;
    statHtml.textContent = `${fmt.format(html.length)} chars`;
    preview.srcdoc = frame(html);
  } catch (e) {
    errEl.hidden = false;
    errEl.textContent = String(e.message ?? e);
  }
}

function loadPreset(name) {
  toonEl.value = PRESETS[name] ?? PRESETS.dev;
  run();
}

async function main() {
  await init();
  toonEl.addEventListener("input", run);
  presetEl.addEventListener("change", () => loadPreset(presetEl.value));
  loadPreset(presetEl.value);
}

main().catch((e) => {
  errEl.hidden = false;
  errEl.textContent = "Failed to initialize WASM: " + e;
});
