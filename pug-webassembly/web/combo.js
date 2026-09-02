// Combo demo: Pug source + locals JSON → HTML, with a preset picker so
// you can see how the same authored shape compares to the rendered HTML
// across a few common patterns (loops, articles, tables, emails).
//
// The pug-wasm `Template` is parsed once per source string and reused
// across keystrokes that only change the locals; the parsed AST is freed
// when the source changes.
import init, { Template } from "./pkg/pug_wasm.js";

const $ = (id) => document.getElementById(id);

const pugEl = $("pug-pane");
const localsEl = $("locals-pane");
const htmlEl = $("html-pane");
const preview = $("preview");
const presetEl = $("preset");
const presetNote = $("preset-note");

const errPugEl = $("err-pug");
const errLocalsEl = $("err-locals");
const errHtmlEl = $("err-html");

const sizeEl = $("size");

const cPug = $("c-pug"), cHtml = $("c-html");
const tPug = $("t-pug"), tHtml = $("t-html");
const rPugHtml = $("r-pug-html");

const fmt = new Intl.NumberFormat();
const approxTokens = (s) => Math.ceil(s.length / 4);

// ─── Presets ────────────────────────────────────────────────────────────

const PRESETS = {
  greetings: {
    note: "loops + conditional + method calls — Pug essentials",
    pug: `doctype html
html
  head
    title= title
  body
    h1.greeting Hello, #{name}!
    ul
      each item, i in items
        li(class=(i === 0 ? "first" : "")) #{i + 1}. #{item.toUpperCase()}
    if items.length > 2
      p.note You have many items.
    else
      p.note Not many items.`,
    locals: {
      title: "Demo",
      name: "World",
      items: ["apples", "oranges", "pears"],
    },
  },

  article: {
    note: "card pattern — class shorthand, interpolation, attr expression",
    pug: `article.card
  header.card-header
    h2.card-title= post.title
    p.byline By #{post.author} · #{post.date}
  .card-body
    p.summary= post.summary
    if post.tags.length
      ul.tags
        each tag in post.tags
          li.tag= tag
  footer.card-footer
    a.cta(href="/posts/" + post.slug) Read more →`,
    locals: {
      post: {
        title: "On the limits of token-oriented serialization",
        author: "Ada Lovelace",
        date: "2026-05-12",
        summary:
          "TOON and friends trade JSON's universality for fewer LLM tokens — here's where the tradeoff pays off, and where it doesn't.",
        tags: ["formats", "llm", "performance"],
        slug: "limits-of-token-formats",
      },
    },
  },

  dashboard: {
    note: "table of users — repetitive markup, big Pug→HTML expansion",
    pug: `section.dashboard
  h1= title
  p.summary #{users.length} members · #{activeCount} active
  table.users
    thead
      tr
        th Name
        th Role
        th Status
    tbody
      each u in users
        tr(class=(u.status === "online" ? "online" : "offline"))
          td= u.name
          td= u.role
          td
            span.dot(class=u.status)
            |  #{u.status}`,
    locals: {
      title: "Team Roster",
      activeCount: 3,
      users: [
        { name: "Ada Lovelace", role: "admin", status: "online" },
        { name: "Alan Turing", role: "user", status: "online" },
        { name: "Grace Hopper", role: "user", status: "away" },
        { name: "Linus Torvalds", role: "user", status: "offline" },
        { name: "Margaret Hamilton", role: "admin", status: "online" },
      ],
    },
  },

  "team-perf": {
    note: "parallel of the json-render.dev Team Performance card",
    pug: `section.card
  h2.card-title= title
  .metric
    .label= revenue.label
    .value #{revenue.prefix}#{revenue.value}
    .change(class=revenue.changeType)= revenue.change
  .chart
    h5 LineGraph
    .sparkline= sparkline
    p.meta #{chartData.length} points · min #{chartMin} · max #{chartMax}
  hr.separator
  each p in progress
    .progress
      .label= p.label
      .bar [#{p.bars}] #{p.value}%`,
    locals: {
      title: "Team Performance",
      revenue: {
        label: "Weekly Revenue",
        value: "12,400",
        prefix: "$",
        change: "+18%",
        changeType: "positive",
      },
      sparkline: "▁▄▂▅▇▆█",
      chartData: [
        { label: "Mon", value: 12 },
        { label: "Tue", value: 28 },
        { label: "Wed", value: 19 },
        { label: "Thu", value: 34 },
        { label: "Fri", value: 45 },
        { label: "Sat", value: 38 },
        { label: "Sun", value: 52 },
      ],
      chartMin: 12,
      chartMax: 52,
      progress: [
        { label: "Deals Closed -- 72%", value: 72, bars: "██████████████░░░░░░" },
        { label: "Retention -- 91%",    value: 91, bars: "██████████████████░░" },
      ],
    },
  },

  shopping: {
    note: "parallel of the json-render.dev Shopping Item card",
    pug: `section.card
  .stack-v
    .image-placeholder
      h5 🖼 #{item.imageAlt}
      p.meta #{item.imageWidth} × #{item.imageHeight}
    .stack-v
      h3= item.name
      p.description= item.description
      .stack-h
        .metric
          .label Price
          .value $#{item.price}
        button.btn.primary Add to Cart`,
    locals: {
      item: {
        name: "Aurora Wireless Headphones",
        description:
          "Over-ear bluetooth headphones with active noise cancellation and 36-hour battery life.",
        price: "249.00",
        imageAlt: "Shopping item product image",
        imageWidth: 400,
        imageHeight: 300,
      },
    },
  },

  email: {
    note: "transactional email — nested doctype + style block + button",
    pug: `doctype html
html
  head
    title= subject
    style.
      body { font-family: sans-serif; max-width: 600px; margin: 24px auto; color: #1a1a1a; }
      .btn { display: inline-block; padding: 10px 18px; background: #2563eb; color: #fff; text-decoration: none; border-radius: 6px; }
      .muted { color: #777; font-size: 13px; }
  body
    h1 Welcome, #{user.name}!
    p Thanks for signing up to #{product}. Confirm your email by clicking the button below — the link expires in 24 hours.
    p
      a.btn(href=user.confirmUrl) Confirm email
    p.muted
      | If you didn't sign up, you can safely ignore this message.
      br
      | This message was sent to #{user.email}.`,
    locals: {
      subject: "Confirm your email",
      product: "Acme Sync",
      user: {
        name: "Ada",
        email: "ada@example.com",
        confirmUrl: "https://acme.example/confirm?token=abcd1234",
      },
    },
  },
};

// ─── Preview wrapping ───────────────────────────────────────────────────

// Full-document outputs (templates that emit `doctype html`) are used
// as-is so we don't double-wrap. Fragments (templates whose root tag
// isn't `<html>`) get a small baseline stylesheet so the rendered HTML
// looks reasonable in the preview — typography, spacing, and styling
// for the common class names used across the presets. The CSS lives in
// JS rather than in each preset so the Pug character count compares
// like-for-like across the dropdown.
function isFullDoc(html) {
  return /^\s*<!doctype/i.test(html) || /^\s*<html\b/i.test(html);
}

function frameDoc(fragment) {
  if (isFullDoc(fragment)) return fragment;
  return `<!doctype html><html><head><meta charset="utf-8"/>
<style>
  body { font: 14px/1.5 system-ui, -apple-system, "Segoe UI", sans-serif; margin: 16px; color: #1a1a1a; max-width: 560px; }
  .card { border: 1px solid #d9d9d9; border-radius: 8px; padding: 14px; margin: 8px 0; background: #fff; }
  .card-title, .card > h2 { font-size: 16px; font-weight: 600; margin: 0 0 10px; }
  .metric { margin: 6px 0; }
  .metric .label { font-size: 12px; color: #666; }
  .metric .value { font-size: 22px; font-weight: 600; line-height: 1.1; }
  .change.positive { color: #16a34a; font-size: 13px; }
  .change.negative { color: #dc2626; font-size: 13px; }
  .chart { margin: 8px 0; }
  .chart h5 { margin: 0 0 4px; font-size: 13px; color: #555; }
  .sparkline { font: 22px ui-monospace, monospace; letter-spacing: 2px; }
  .meta { font-size: 12px; color: #888; margin: 2px 0; }
  .separator { border: none; border-top: 1px solid #e5e7eb; margin: 12px 0; }
  .progress { margin: 4px 0; }
  .progress .label { font-size: 12px; color: #555; }
  .progress .bar { font: 13px ui-monospace, monospace; color: #1a1a1a; }
  .btn { padding: 8px 14px; border-radius: 6px; border: 1px solid #888; background: #f5f5f5; cursor: pointer; font: inherit; }
  .btn.primary { background: #2563eb; color: #fff; border-color: #1d4ed8; }
  .stack-v { display: flex; flex-direction: column; gap: 10px; }
  .stack-h { display: flex; flex-direction: row; gap: 12px; align-items: center; justify-content: space-between; }
  .image-placeholder { border: 2px dashed #ccc; border-radius: 8px; padding: 24px; background: #fafafa; text-align: center; }
  .image-placeholder h5 { margin: 0; font-size: 14px; color: #555; }
  .description { color: #333; font-size: 13px; margin: 4px 0; }
  table.users { border-collapse: collapse; margin: 8px 0; }
  table.users th, table.users td { border: 1px solid #d9d9d9; padding: 4px 8px; text-align: left; font-size: 13px; }
  table.users tr.online td { font-weight: 600; }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; padding: 0; margin: 6px 0; list-style: none; }
  .tag { background: #e5e7eb; padding: 2px 8px; border-radius: 999px; font-size: 12px; }
  .cta { color: #2563eb; }
  .summary { color: #555; font-size: 13px; }
  .greeting { color: #1d4ed8; }
  .note { color: #666; font-style: italic; }
  .first { font-weight: 600; }
</style></head><body>${fragment}</body></html>`;
}

// ─── Defense-in-depth sanitizer (mirrors app.js) ────────────────────────

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

// ─── UI plumbing ────────────────────────────────────────────────────────

function showErr(el, msg) {
  el.hidden = false;
  el.textContent = msg;
}
function clearErr(el) {
  el.hidden = true;
  el.textContent = "";
}

function setRatio(el, a, b) {
  // a = html, b = pug. Show how much HTML grows vs the pug source.
  if (a === 0 || b === 0) {
    el.textContent = "—";
    delete el.dataset.sign;
    return;
  }
  const ratio = a / b;
  el.textContent = `×${ratio.toFixed(2)}`;
  el.dataset.sign = ratio > 1 ? "grows" : ratio < 1 ? "saves" : "";
}

function updateStats(pug, html) {
  cPug.textContent = fmt.format(pug.length);
  cHtml.textContent = fmt.format(html.length);
  tPug.textContent = fmt.format(approxTokens(pug));
  tHtml.textContent = fmt.format(approxTokens(html));
  setRatio(rPugHtml, html.length, pug.length);
}

// Cache the parsed Template so changing only the locals doesn't re-parse.
let cached = { source: null, template: null };
function getTemplate(source) {
  if (cached.source === source && cached.template) return cached.template;
  if (cached.template) cached.template.free();
  cached = { source, template: new Template(source) };
  return cached.template;
}

function render() {
  let html = "";
  try {
    const tmpl = getTemplate(pugEl.value);
    clearErr(errPugEl);
    try {
      html = tmpl.render(localsEl.value);
      clearErr(errLocalsEl);
      clearErr(errHtmlEl);
    } catch (e) {
      // Locals parse error or render-time eval error
      showErr(errLocalsEl, String(e?.message ?? e));
    }
  } catch (e) {
    showErr(errPugEl, String(e?.message ?? e));
  }

  if (html) {
    const safe = sanitizeHtml(html);
    htmlEl.textContent = safe;
    preview.srcdoc = frameDoc(safe);
  } else {
    htmlEl.textContent = "";
    preview.srcdoc = "";
  }

  updateStats(pugEl.value, html);
}

let debounceId;
function schedule() {
  clearTimeout(debounceId);
  debounceId = setTimeout(render, 120);
}

function loadPreset(name) {
  const p = PRESETS[name];
  if (!p) return;
  pugEl.value = p.pug;
  localsEl.value = JSON.stringify(p.locals, null, 2);
  presetNote.textContent = p.note;
  render();
}

async function main() {
  await init();
  try {
    const res = await fetch("./pkg/pug_wasm_bg.wasm", { method: "HEAD" });
    const sz = res.headers.get("content-length");
    if (sz) sizeEl.textContent = `· WASM ${(sz / 1024).toFixed(1)} KB`;
  } catch (_) { /* ignore */ }

  pugEl.addEventListener("input", schedule);
  localsEl.addEventListener("input", schedule);
  presetEl.addEventListener("change", () => loadPreset(presetEl.value));

  loadPreset(presetEl.value);
}

main().catch((e) => {
  errHtmlEl.hidden = false;
  errHtmlEl.textContent = "Failed to initialize WASM: " + e;
});
