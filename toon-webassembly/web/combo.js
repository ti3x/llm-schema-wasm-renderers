// Combo demo: TOON ⇄ JSON, then JSON → HTML via json-render-wasm.
//
// Supports two JSON spec shapes:
//
//  • Simple (my json-render-wasm native form):
//      { tag, props, children: [ ... ] }
//
//  • json-render.dev form (https://json-render.dev/):
//      { root, state, elements: { id: {type, props, children: [id...]} } }
//      with $state: "/path" bindings and $template: "${/path}" strings.
//
// The pipeline keeps the original spec in the JSON pane so TOON is
// computed against the authentic bytes. A JS-side adapter inlines the
// json-render.dev form into the simple form before sending it to the
// WASM renderer. Components not in my renderer's small catalog (Stack,
// Metric, LineGraph, Progress, Separator) are synthesized from
// Container + Text + Heading so the visual story still works.
import initToon, { json_to_toon, toon_to_json } from "./pkg/toon_wasm.js";
import initRender, { compile } from "./pkg-render/json_render_wasm.js";

const $ = (id) => document.getElementById(id);

const toonEl = $("toon-pane");
const jsonEl = $("json-pane");
const htmlEl = $("html-pane");
const preview = $("preview");
const presetEl = $("preset");
const presetNote = $("preset-note");

const errToonEl = $("err-toon");
const errJsonEl = $("err-json");
const errHtmlEl = $("err-html");

const cToon = $("c-toon"),
  cJson = $("c-json"),
  cHtml = $("c-html");
const tToon = $("t-toon"),
  tJson = $("t-json"),
  tHtml = $("t-html");
const rToonJson = $("r-toon-json");
const rJsonHtml = $("r-json-html");

const fmt = new Intl.NumberFormat();
const approxTokens = (s) => Math.ceil(s.length / 4);

// ─── Presets ─────────────────────────────────────────────────────────────

const PRESETS = {
  dashboard: {
    note: "json-render.dev format · indirected nodes + $state bindings",
    json: {
      root: "card",
      state: {
        chartData: [
          { label: "Mon", value: 12 },
          { label: "Tue", value: 28 },
          { label: "Wed", value: 19 },
          { label: "Thu", value: 34 },
          { label: "Fri", value: 45 },
          { label: "Sat", value: 38 },
          { label: "Sun", value: 52 },
        ],
      },
      elements: {
        card: {
          type: "Card",
          props: { title: "Team Performance", maxWidth: "sm", centered: true },
          children: ["m1", "chart", "sep", "p1", "p2"],
        },
        m1: {
          type: "Metric",
          props: {
            label: "Weekly Revenue",
            value: "12,400",
            prefix: "$",
            change: "+18%",
            changeType: "positive",
          },
        },
        chart: {
          type: "LineGraph",
          props: { data: { $state: "/chartData" } },
        },
        sep: { type: "Separator", props: {} },
        p1: { type: "Progress", props: { value: 72, label: "Deals Closed -- 72%" } },
        p2: { type: "Progress", props: { value: 91, label: "Retention -- 91%" } },
      },
    },
  },
  shopping: {
    note: "json-render.dev format · Stack layout + $state + $template",
    json: {
      root: "shopping-card",
      state: {
        item: {
          name: "Aurora Wireless Headphones",
          description:
            "Over-ear bluetooth headphones with active noise cancellation and 36-hour battery life.",
          price: "249.00",
        },
      },
      elements: {
        "shopping-card": {
          type: "Card",
          props: { maxWidth: "sm", centered: false },
          children: ["card-stack"],
        },
        "card-stack": {
          type: "Stack",
          props: { direction: "vertical", gap: "md" },
          children: ["item-image", "item-details"],
        },
        "item-image": {
          type: "Image",
          props: { alt: "Shopping item product image", width: 400, height: 300 },
        },
        "item-details": {
          type: "Stack",
          props: { direction: "vertical", gap: "sm" },
          children: ["item-name", "item-description", "price-section"],
        },
        "item-name": {
          type: "Heading",
          props: { text: { $state: "/item/name" }, level: "h3" },
        },
        "item-description": {
          type: "Text",
          props: { text: { $state: "/item/description" }, variant: "body" },
        },
        "price-section": {
          type: "Stack",
          props: {
            direction: "horizontal",
            gap: "md",
            justify: "between",
            align: "center",
          },
          children: ["price-display", "add-to-cart-button"],
        },
        "price-display": {
          type: "Metric",
          props: {
            label: "Price",
            value: { $template: "${/item/price}" },
            prefix: "$",
          },
        },
        "add-to-cart-button": {
          type: "Button",
          props: { label: "Add to Cart", variant: "primary" },
          on: { press: { action: "buttonClick", params: { message: "Added to cart!" } } },
        },
      },
    },
  },
  table: {
    note: "simple format · my json-render-wasm native shape, no indirection",
    json: {
      tag: "Card",
      props: { title: "Active Members" },
      children: [
        { tag: "Heading", props: { level: 2, value: "Team roster" } },
        {
          tag: "Table",
          props: {
            columns: [
              { header: "Name", field: "name" },
              { header: "Role", field: "role" },
              { header: "Status", field: "status" },
            ],
            rows: [
              { name: "Ada Lovelace", role: "admin", status: "online" },
              { name: "Alan Turing", role: "user", status: "online" },
              { name: "Grace Hopper", role: "user", status: "away" },
            ],
          },
        },
        { tag: "Button", props: { label: "Invite member", variant: "primary" } },
      ],
    },
  },
};

// ─── Adapter: json-render.dev format → my simple format ──────────────────

function isIndirected(spec) {
  return (
    spec &&
    typeof spec === "object" &&
    !Array.isArray(spec) &&
    typeof spec.root === "string" &&
    spec.elements &&
    typeof spec.elements === "object"
  );
}

function jsonPointer(root, path) {
  if (!path || path === "/") return root;
  const segments = path.replace(/^\//, "").split("/").map(decodePointer);
  let cur = root;
  for (const seg of segments) {
    if (cur == null) return undefined;
    cur = cur[seg];
  }
  return cur;
}
function decodePointer(s) {
  return s.replace(/~1/g, "/").replace(/~0/g, "~");
}

function resolveBindings(value, state) {
  if (value == null || typeof value !== "object") return value;
  if (Array.isArray(value)) return value.map((v) => resolveBindings(v, state));
  if ("$state" in value && typeof value.$state === "string") {
    return jsonPointer(state, value.$state);
  }
  if ("$template" in value && typeof value.$template === "string") {
    return value.$template.replace(/\$\{([^}]+)\}/g, (_, p) => {
      const v = jsonPointer(state, p);
      return v == null ? "" : String(v);
    });
  }
  const out = {};
  for (const [k, v] of Object.entries(value)) {
    out[k] = resolveBindings(v, state);
  }
  return out;
}

function sparkline(values) {
  if (!values || !values.length) return "";
  const bars = "▁▂▃▄▅▆▇█";
  const max = Math.max(...values);
  const min = Math.min(...values);
  const span = max - min || 1;
  return values
    .map((v) => bars[Math.min(bars.length - 1, Math.floor(((v - min) / span) * (bars.length - 1)))])
    .join("");
}

function progressBar(pct, width = 20) {
  const v = Math.max(0, Math.min(100, Number(pct) || 0));
  const filled = Math.round((v / 100) * width);
  return "[" + "█".repeat(filled) + "░".repeat(width - filled) + `] ${v}%`;
}

function levelFromString(s) {
  const n = parseInt(String(s).replace(/^h/i, ""), 10);
  return Number.isFinite(n) && n >= 1 && n <= 6 ? n : 2;
}

function adapt(spec) {
  if (!isIndirected(spec)) return spec; // already simple form
  const state = spec.state ?? {};
  const elements = spec.elements;

  function buildById(id) {
    const node = elements[id];
    if (!node) {
      return { tag: "Text", props: { value: `[missing element: ${id}]` } };
    }
    return buildNode(node);
  }

  function buildNode(node) {
    const rawProps = node.props ?? {};
    const props = {};
    for (const [k, v] of Object.entries(rawProps)) {
      props[k] = resolveBindings(v, state);
    }
    const childIds = Array.isArray(node.children) ? node.children : [];
    const children = childIds.map(buildById);

    switch (node.type) {
      case "Card":
        return {
          tag: "Card",
          props: props.title != null ? { title: String(props.title) } : {},
          children,
        };
      case "Container":
      case "Stack":
        return { tag: "Container", children };
      case "Heading":
        return {
          tag: "Heading",
          props: {
            level: levelFromString(props.level ?? 2),
            value: String(props.text ?? props.value ?? ""),
          },
        };
      case "Text":
        return { tag: "Text", props: { value: String(props.text ?? props.value ?? "") } };
      case "Button":
        return {
          tag: "Button",
          props: {
            label: String(props.label ?? ""),
            variant: props.variant ? String(props.variant) : "",
          },
        };
      case "Image": {
        // Parent CSP blocks remote images in the srcdoc iframe; emit a
        // styled Container with alt text instead of a real <img>.
        const alt = String(props.alt ?? "Image");
        const w = props.width ? `${props.width}` : "—";
        const h = props.height ? `${props.height}` : "—";
        return {
          tag: "Container",
          children: [
            { tag: "Heading", props: { level: 5, value: "🖼 " + alt } },
            { tag: "Text", props: { value: `${w} × ${h}` } },
          ],
        };
      }
      case "Separator":
        return {
          tag: "Container",
          children: [{ tag: "Text", props: { value: "──────────" } }],
        };
      case "Metric": {
        const prefix = props.prefix != null ? String(props.prefix) : "";
        const value = props.value != null ? String(props.value) : "";
        const change = props.change != null ? String(props.change) : "";
        const label = props.label != null ? String(props.label) : "";
        const kids = [];
        if (label) kids.push({ tag: "Text", props: { value: label } });
        kids.push({ tag: "Heading", props: { level: 3, value: `${prefix}${value}` } });
        if (change) kids.push({ tag: "Text", props: { value: change } });
        return { tag: "Container", children: kids };
      }
      case "LineGraph": {
        const data = props.data ?? [];
        const values = Array.isArray(data) ? data.map((p) => Number(p?.value ?? p) || 0) : [];
        return {
          tag: "Container",
          children: [
            { tag: "Heading", props: { level: 4, value: "LineGraph" } },
            { tag: "Text", props: { value: sparkline(values) || "(no data)" } },
            {
              tag: "Text",
              props: {
                value: values.length
                  ? `${values.length} points · min ${Math.min(...values)} · max ${Math.max(...values)}`
                  : "",
              },
            },
          ],
        };
      }
      case "Progress": {
        const label = props.label != null ? String(props.label) : "";
        const v = Number(props.value) || 0;
        const kids = [];
        if (label) kids.push({ tag: "Text", props: { value: label } });
        kids.push({ tag: "Text", props: { value: progressBar(v) } });
        return { tag: "Container", children: kids };
      }
      default:
        // Unknown component — render as a placeholder so the page still loads.
        return {
          tag: "Container",
          children: [
            { tag: "Heading", props: { level: 5, value: `[${node.type}]` } },
            ...children,
          ],
        };
    }
  }

  return buildById(spec.root);
}

// ─── UI plumbing ────────────────────────────────────────────────────────

function showErr(el, msg) {
  el.hidden = false;
  el.textContent = msg;
}
function clearErr(el) {
  el.hidden = true;
  el.textContent = "";
}

function setRatio(el, a, b, smallerIsBetter) {
  if (a === 0 || b === 0) {
    el.textContent = "—";
    delete el.dataset.sign;
    return;
  }
  if (smallerIsBetter) {
    const pct = ((b - a) / b) * 100;
    el.textContent = pct > 0 ? `−${pct.toFixed(0)}%` : `+${(-pct).toFixed(0)}%`;
    el.dataset.sign = pct > 0 ? "saves" : pct < 0 ? "grows" : "";
  } else {
    const ratio = a / b;
    el.textContent = `×${ratio.toFixed(2)}`;
    el.dataset.sign = ratio > 1 ? "grows" : ratio < 1 ? "saves" : "";
  }
}

function updateStats(toon, json, html) {
  cToon.textContent = fmt.format(toon.length);
  cJson.textContent = fmt.format(json.length);
  cHtml.textContent = fmt.format(html.length);
  tToon.textContent = fmt.format(approxTokens(toon));
  tJson.textContent = fmt.format(approxTokens(json));
  tHtml.textContent = fmt.format(approxTokens(html));
  setRatio(rToonJson, toon.length, json.length, true);
  setRatio(rJsonHtml, html.length, json.length, false);
}

function frameDoc(fragment) {
  return `<!doctype html><html><head><meta charset="utf-8"/>
<style>
  body { font: 14px system-ui, sans-serif; margin: 16px; color: #1a1a1a; }
  .jr-card { border: 1px solid #d9d9d9; border-radius: 8px; padding: 12px; margin: 8px 0; }
  .jr-card-title { font-weight: 600; font-size: 16px; margin-bottom: 8px; }
  .jr-container { display: flex; flex-direction: column; gap: 8px; margin: 4px 0; }
  .jr-heading { margin: 4px 0; }
  .jr-text { display: block; }
  .jr-list { padding-left: 18px; }
  .jr-btn { padding: 6px 12px; border-radius: 6px; border: 1px solid #888; background: #f5f5f5; cursor: pointer; align-self: flex-start; }
  .jr-btn-primary { background: #2563eb; color: #fff; border-color: #1d4ed8; }
  .jr-btn-secondary { background: #e5e7eb; }
  .jr-btn-danger { background: #dc2626; color: #fff; border-color: #b91c1c; }
  .jr-btn-ghost { background: transparent; border-color: transparent; }
  .jr-input { padding: 6px 8px; border: 1px solid #ccc; border-radius: 6px; min-width: 200px; }
  .jr-table { border-collapse: collapse; margin: 4px 0; }
  .jr-table th, .jr-table td { border: 1px solid #d9d9d9; padding: 4px 8px; text-align: left; }
  .jr-link { color: #2563eb; }
  .jr-image { max-width: 100%; height: auto; border-radius: 6px; }
</style></head><body>${fragment}</body></html>`;
}

const opts = JSON.stringify({ delimiter: "comma", indent: 2, strict: true, coerceTypes: true });

let silent = false;
let debounceId;

function schedule(source) {
  clearTimeout(debounceId);
  debounceId = setTimeout(() => runChain(source), 120);
}

function runChain(source) {
  if (silent) return;
  let toon = toonEl.value;
  let json = jsonEl.value;
  let html = "";

  try {
    silent = true;
    if (source === "toon") {
      json = toon_to_json(toon, opts);
      jsonEl.value = json;
    } else if (source === "json" || source === "preset" || source === "init") {
      toon = json_to_toon(json, opts);
      toonEl.value = toon;
    }
    clearErr(errToonEl);
    clearErr(errJsonEl);
  } catch (e) {
    const msg = String(e?.message ?? e);
    if (source === "toon") showErr(errToonEl, msg);
    else showErr(errJsonEl, msg);
    silent = false;
    updateStats(toon, json, html);
    return;
  } finally {
    silent = false;
  }

  // Step 2: parse JSON, adapt if json-render.dev form, render via WASM.
  try {
    let parsed;
    try {
      parsed = JSON.parse(json);
    } catch (e) {
      throw new Error("JSON parse failed: " + e.message);
    }
    const simple = adapt(parsed);
    const simpleJson = JSON.stringify(simple);
    html = compile(simpleJson, "");
    htmlEl.textContent = html;
    preview.srcdoc = frameDoc(html);
    clearErr(errHtmlEl);
  } catch (e) {
    showErr(errHtmlEl, String(e?.message ?? e));
    htmlEl.textContent = "";
    preview.srcdoc = "";
    html = "";
  }

  updateStats(toon, json, html);
}

function loadPreset(name) {
  const p = PRESETS[name];
  if (!p) return;
  jsonEl.value = JSON.stringify(p.json, null, 2);
  presetNote.textContent = p.note;
  runChain("preset");
}

async function main() {
  await Promise.all([initToon(), initRender()]);
  toonEl.addEventListener("input", () => schedule("toon"));
  jsonEl.addEventListener("input", () => schedule("json"));
  presetEl.addEventListener("change", () => loadPreset(presetEl.value));
  loadPreset(presetEl.value);
}

main().catch((e) => {
  errHtmlEl.hidden = false;
  errHtmlEl.textContent = "Failed to initialize WASM: " + e;
});
