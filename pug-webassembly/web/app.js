// Recommended consumer pattern for backend-supplied pug:
//   1. Parse the template once into a `Template` (cached AST in WASM).
//   2. Render it against per-request locals.
//   3. Mount the resulting HTML into a `<iframe sandbox srcdoc=...>`.
//
// `sandbox=""` (the default below) blocks scripts entirely; if you need
// the rendered page to run its own scripts, switch to
// `sandbox="allow-scripts"` — but NEVER combine `allow-scripts` with
// `allow-same-origin` (that's a documented sandbox escape).
import init, { Template } from "./pkg/pug_wasm.js";

const $ = (id) => document.getElementById(id);

const srcEl  = $("src");
const dataEl = $("data");
const outEl  = $("out");
const errEl  = $("err");
const preview = $("preview");
const allowScripts = $("allow-scripts");
const stripScriptsEl = $("strip-scripts");
const sizeEl = $("size");
const pugCharsEl = $("pug-chars");
const dataCharsEl = $("data-chars");
const htmlCharsEl = $("html-chars");
const savedTplEl = $("saved-tpl");
const savedFullEl = $("saved-full");

const fmt = new Intl.NumberFormat();
// % smaller `part` is than `whole` (positive ⇒ source is smaller than HTML).
const pctSaved = (whole, part) => (whole > 0 ? ((whole - part) / whole) * 100 : 0);
const signOf = (n) => (n > 0 ? "pos" : n < 0 ? "neg" : "zero");

function updateStats(pug, data, html) {
  const p = pug.length;
  const d = data.length;
  const h = html.length;
  // Two comparisons, since fairness depends on the scenario:
  //  • template only  — the template is reused across many renders and the
  //    data arrives from a separate call / local stack (template amortized).
  //  • template + data — a single self-contained render pays for both.
  const savedTpl = pctSaved(h, p);
  const savedFull = pctSaved(h, p + d);
  pugCharsEl.textContent = fmt.format(p);
  dataCharsEl.textContent = fmt.format(d);
  htmlCharsEl.textContent = fmt.format(h);
  savedTplEl.textContent = `${savedTpl.toFixed(1)}%`;
  savedFullEl.textContent = `${savedFull.toFixed(1)}%`;
  savedTplEl.parentElement.dataset.sign = signOf(savedTpl);
  savedFullEl.parentElement.dataset.sign = signOf(savedFull);
}

/**
 * Strip `<script>` tags, inline event handlers (`onclick=` etc.), and
 * `javascript:` URIs from rendered HTML. Uses the browser's HTML parser
 * (no regex), so it correctly handles weird whitespace, attributes, and
 * comment-hidden scripts.
 *
 * Not a full sanitizer — for that use DOMPurify. This is intended as
 * defense-in-depth alongside the sandboxed iframe, not a replacement.
 */
function sanitizeHtml(html) {
  const isFullDoc = /^\s*<!doctype/i.test(html) || /^\s*<html\b/i.test(html);
  const doc = new DOMParser().parseFromString(html, "text/html");

  doc.querySelectorAll("script,noscript").forEach((n) => n.remove());

  for (const el of doc.querySelectorAll("*")) {
    // Iterate over a snapshot — removeAttribute mutates the live list.
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

/** Cached parsed template. Reused whenever the source string is unchanged. */
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
  updateStats(srcEl.value, dataEl.value, "");
}

function render() {
  try {
    const tmpl = getTemplate(srcEl.value);
    let html = tmpl.render(dataEl.value);
    if (stripScriptsEl.checked) html = sanitizeHtml(html);

    outEl.textContent = html;
    errEl.hidden = true;
    errEl.textContent = "";
    updateStats(srcEl.value, dataEl.value, html);

    // Default-safe: empty sandbox attribute blocks scripts entirely.
    preview.setAttribute(
      "sandbox",
      allowScripts.checked ? "allow-scripts" : ""
    );
    preview.srcdoc = html;
  } catch (e) {
    showError(String(e?.message ?? e));
  }
}

let debounce;
const schedule = () => {
  clearTimeout(debounce);
  debounce = setTimeout(render, 120);
};

async function main() {
  await init();

  try {
    const res = await fetch("./pkg/pug_wasm_bg.wasm", { method: "HEAD" });
    const sz = res.headers.get("content-length");
    if (sz) sizeEl.textContent = `· WASM ${(sz / 1024).toFixed(1)} KB`;
  } catch (_) { /* ignore */ }

  srcEl.addEventListener("input", schedule);
  dataEl.addEventListener("input", schedule);
  allowScripts.addEventListener("change", render);
  stripScriptsEl.addEventListener("change", render);
  render();
}

main().catch((e) => showError("Failed to initialize WASM: " + e));
