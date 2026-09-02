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
const htmlCharsEl = $("html-chars");
const deltaCharsEl = $("delta-chars");
const deltaPctEl = $("delta-pct");

const fmt = new Intl.NumberFormat();
const signed = (n) => (n > 0 ? `+${fmt.format(n)}` : fmt.format(n));

function updateStats(pug, html) {
  const p = pug.length;
  const h = html.length;
  const diff = h - p;            // positive ⇒ HTML is bigger than pug
  // "% saved" = how much shorter pug is vs the equivalent HTML.
  // 0% when they're equal, 100% when html is much bigger than pug.
  const pct = h > 0 ? (diff / h) * 100 : 0;
  pugCharsEl.textContent = fmt.format(p);
  htmlCharsEl.textContent = fmt.format(h);
  deltaCharsEl.textContent = signed(diff);
  deltaPctEl.textContent = `${pct.toFixed(1)}%`;
  deltaCharsEl.parentElement.dataset.sign = diff > 0 ? "pos" : diff < 0 ? "neg" : "zero";
  deltaPctEl.parentElement.dataset.sign = diff > 0 ? "pos" : diff < 0 ? "neg" : "zero";
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
  updateStats(srcEl.value, "");
}

function render() {
  try {
    const tmpl = getTemplate(srcEl.value);
    let html = tmpl.render(dataEl.value);
    if (stripScriptsEl.checked) html = sanitizeHtml(html);

    outEl.textContent = html;
    errEl.hidden = true;
    errEl.textContent = "";
    updateStats(srcEl.value, html);

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
