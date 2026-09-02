// Recommended consumer pattern:
//   1. Parse the spec once into a `Template` (cached AST in WASM).
//   2. Render it against per-frame state.
//   3. Mount the resulting HTML into a `<iframe sandbox srcdoc=...>`.
//
// `sandbox=""` (the default below) blocks scripts entirely; if you need
// the rendered page to run its own scripts, switch to
// `sandbox="allow-scripts"` — but NEVER combine `allow-scripts` with
// `allow-same-origin` (that's a documented sandbox escape).
import init, { Template } from "./pkg/json_render_wasm.js";

const $ = (id) => document.getElementById(id);

const specEl = $("spec");
const stateEl = $("state");
const outEl = $("out");
const errEl = $("err");
const preview = $("preview");
const allowScripts = $("allow-scripts");
const stripScriptsEl = $("strip-scripts");
const sizeEl = $("size");
const specCharsEl = $("spec-chars");
const htmlCharsEl = $("html-chars");
const deltaCharsEl = $("delta-chars");
const promptEl = $("prompt");
const endpointEl = $("endpoint");
const genBtn = $("gen-btn");

const fmt = new Intl.NumberFormat();
const signed = (n) => (n > 0 ? `+${fmt.format(n)}` : fmt.format(n));

function updateStats(spec, html) {
  const s = spec.length;
  const h = html.length;
  const diff = h - s;
  specCharsEl.textContent = fmt.format(s);
  htmlCharsEl.textContent = fmt.format(h);
  deltaCharsEl.textContent = signed(diff);
  deltaCharsEl.parentElement.dataset.sign = diff > 0 ? "pos" : diff < 0 ? "neg" : "zero";
}

/**
 * Strip `<script>` tags, inline event handlers, and `javascript:` URIs
 * from rendered HTML. Uses the browser's HTML parser (no regex). This
 * is defense-in-depth — the renderer never emits these in the first
 * place, but the toggle lets you verify that on hostile specs.
 */
function sanitizeHtml(html) {
  const isFullDoc = /^\s*<!doctype/i.test(html) || /^\s*<html\b/i.test(html);
  const doc = new DOMParser().parseFromString(html, "text/html");
  doc.querySelectorAll("script,noscript").forEach((n) => n.remove());
  for (const el of doc.querySelectorAll("*")) {
    for (const attr of [...el.attributes]) {
      const name = attr.name.toLowerCase();
      if (name.startsWith("on")) {
        el.removeAttribute(attr.name);
        continue;
      }
      if (
        (name === "href" || name === "src" || name === "xlink:href" ||
          name === "formaction" || name === "action") &&
        /^\s*javascript:/i.test(attr.value)
      ) {
        el.removeAttribute(attr.name);
      }
    }
  }
  return isFullDoc
    ? "<!DOCTYPE html>\n" + doc.documentElement.outerHTML
    : doc.body.innerHTML;
}

/**
 * Wrap the rendered fragment in a minimal page so the iframe gets some
 * baseline styling and a viewport meta. The rendered fragment is
 * already HTML-escaped by the WASM renderer.
 */
function frameDoc(fragment) {
  return `<!doctype html><html><head><meta charset="utf-8"/>
<style>
  body { font: 14px system-ui, sans-serif; margin: 16px; color: #1a1a1a; }
  .jr-card { border: 1px solid #d9d9d9; border-radius: 8px; padding: 12px; margin: 8px 0; }
  .jr-card-title { font-weight: 600; margin-bottom: 8px; }
  .jr-list { padding-left: 18px; }
  .jr-btn { padding: 6px 12px; border-radius: 6px; border: 1px solid #888; background: #f5f5f5; cursor: pointer; }
  .jr-btn-primary { background: #2563eb; color: #fff; border-color: #1d4ed8; }
  .jr-btn-secondary { background: #e5e7eb; }
  .jr-btn-danger { background: #dc2626; color: #fff; border-color: #b91c1c; }
  .jr-btn-ghost { background: transparent; border-color: transparent; }
  .jr-input { padding: 6px 8px; border: 1px solid #ccc; border-radius: 6px; min-width: 200px; }
  .jr-table { border-collapse: collapse; }
  .jr-table th, .jr-table td { border: 1px solid #d9d9d9; padding: 4px 8px; text-align: left; }
  .jr-link { color: #2563eb; }
</style></head><body>${fragment}</body></html>`;
}

let cached = { source: null, template: null };

function getTemplate(source) {
  if (cached.source === source && cached.template) return cached.template;
  if (cached.template) cached.template.free();
  cached = { source, template: new Template(source) };
  return cached.template;
}

function showError(msg) {
  outEl.textContent = "";
  errEl.hidden = false;
  errEl.textContent = msg;
  preview.srcdoc = "";
  updateStats(specEl.value, "");
}

function render() {
  try {
    const tmpl = getTemplate(specEl.value);
    let html = tmpl.render(stateEl.value);
    if (stripScriptsEl.checked) html = sanitizeHtml(html);
    outEl.textContent = html;
    errEl.hidden = true;
    errEl.textContent = "";
    updateStats(specEl.value, html);
    preview.setAttribute(
      "sandbox",
      allowScripts.checked ? "allow-scripts" : ""
    );
    preview.srcdoc = frameDoc(html);
  } catch (e) {
    showError(String(e?.message ?? e));
  }
}

let debounce;
const schedule = () => {
  clearTimeout(debounce);
  debounce = setTimeout(render, 120);
};

async function generate() {
  const endpoint = endpointEl.value.trim();
  if (!endpoint) {
    showError("Set an LLM endpoint URL first (anything that returns { spec: <object> }).");
    return;
  }
  const prompt = promptEl.value.trim();
  if (!prompt) return;
  genBtn.disabled = true;
  genBtn.textContent = "…";
  try {
    const res = await fetch(endpoint, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ prompt }),
    });
    if (!res.ok) throw new Error(`endpoint returned ${res.status}`);
    const data = await res.json();
    const spec = data.spec ?? data;
    specEl.value = JSON.stringify(spec, null, 2);
    render();
  } catch (e) {
    showError("Generation failed: " + (e?.message ?? e));
  } finally {
    genBtn.disabled = false;
    genBtn.textContent = "Generate";
  }
}

async function main() {
  await init();
  try {
    const res = await fetch("./pkg/json_render_wasm_bg.wasm", { method: "HEAD" });
    const sz = res.headers.get("content-length");
    if (sz) sizeEl.textContent = `· WASM ${(sz / 1024).toFixed(1)} KB`;
  } catch (_) { /* ignore */ }

  specEl.addEventListener("input", schedule);
  stateEl.addEventListener("input", schedule);
  allowScripts.addEventListener("change", render);
  stripScriptsEl.addEventListener("change", render);
  genBtn.addEventListener("click", generate);
  render();
}

main().catch((e) => showError("Failed to initialize WASM: " + e));
