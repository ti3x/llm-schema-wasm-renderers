# pug → html · WebAssembly

A security-focused pug renderer for the browser. Pure Rust, no embedded
JavaScript engine, compiled to a 233 KB WebAssembly module.

## Why

[pug.js](https://github.com/pugjs/pug) compiles templates by building a JS
function-body string and handing it to `new Function(...)`. Any
attacker-influenceable template fragment becomes arbitrary host-context
code — the SSTI/RCE class behind multiple pug CVEs.

This project takes the same design trade-off that the
[Go](https://github.com/Joker/jade), [Python](https://pypi.org/project/pypugjs/),
[Java](https://github.com/neuland/jade4j) and [PHP](https://github.com/pug-php/pug)
ports made years ago: replace JS-expression semantics with a small
host-language expression evaluator over plain data. Result: there is no
arbitrary-execution primitive at all, so untrusted templates cannot
escape into the page.

## Run it

```bash
./build.sh
cd web
python3 -m http.server 8765
# then open http://localhost:8765
```

## Canonical consumer pattern: backend-supplied pug

If your application fetches pug source from a backend and renders it
per request, parse once and render many times. Mount the resulting HTML
into a sandboxed iframe — that way, even if any part of the template
string is attacker-influenced, scripts in the output cannot escape into
the page origin.

```js
import init, { Template } from "./pkg/pug_wasm.js";
await init();

async function renderRequest(iframe) {
  const { template, data } = await fetch("/api/render").then(r => r.json());

  // Parse the template once (Rust caches the AST in WASM memory).
  const tmpl = new Template(template);
  try {
    const html = tmpl.render(JSON.stringify(data));

    // Default-safe: `sandbox=""` blocks scripts entirely.
    // Use `sandbox="allow-scripts"` only if you trust the template
    // author to ship JS — and NEVER add `allow-same-origin`.
    iframe.setAttribute("sandbox", "");
    iframe.srcdoc = html;
  } finally {
    tmpl.free();  // or rely on `[Symbol.dispose]` via `using`
  }
}
```

For templates that are rendered with multiple data shapes during a
session, keep the `Template` around and call `.render(...)` for each:

```js
const tmpl = new Template(source);
for (const data of perTurnLocals) {
  iframe.srcdoc = tmpl.render(JSON.stringify(data));
}
tmpl.free();
```

`compile(source, locals_json)` is also exported for one-shot renders.

## Tests

```bash
cargo test
```

51 tests covering syntax, escaping, iteration limits, and a battery of
hostile-input rejections (`eval`, `function`, `=>`, `constructor`,
`__proto__`, `new`, `this`, bare function calls, assignment, methods
outside the whitelist).

## What's supported

- Tags, class/id shorthand, attributes
- Text + `#{escaped}` / `!{raw}` interpolation
- Buffered code: `p= expr`, `p!= expr`
- Conditionals: `if` / `else if` / `else` / `unless`
- Iteration: `each item in arr`, `each item, idx in obj`
- Code lines: `- var x = expr` (`let`/`const` too)
- Comments: `//` (visible) and `//-` (silent)
- Block text: `tag.\n  ...`
- Void elements, doctype, attribute auto-escaping

Out of scope for the MVP: mixins, `extends`/`block`, `include`, filters,
`case`/`when`.

## Expression language

Numbers, strings, booleans, `null`, identifiers (resolved against the
JSON locals), arrays, objects, full operator precedence including ternary,
member access `.x` / `["x"]`, and a whitelist of methods:

| receiver | methods                                              |
|----------|------------------------------------------------------|
| string   | `.length`, `.toUpperCase()`, `.toLowerCase()`, `.trim()`, `.includes(s)` |
| array    | `.length`, `.includes(x)`, `.join(sep)`              |
| object   | `.hasOwnProperty(k)`                                 |

Hard rejects at parse time: `function`, `=>`, `new`, `delete`, `typeof`,
`instanceof`, `this`, `eval`, `constructor`, `__proto__`, `prototype`,
assignment, bare function calls.

## Defense in depth

1. **No execution primitive** — the grammar has no way to express
   function definitions or calls outside the whitelist.
2. **Auto-escape** — interpolations and attribute values are HTML-escaped
   by default; raw output requires explicit `!{...}` or `!=`.
3. **WASM sandbox** — even a memory-safety bug in the renderer is
   confined to WASM linear memory; the module imports nothing from the
   host except text in and text out.
4. **Sandboxed preview** — the playground's preview `<iframe>` uses the
   `sandbox` attribute. By default it disables scripts entirely; an
   opt-in toggle adds `allow-scripts` (never combined with
   `allow-same-origin`).
5. **`#![forbid(unsafe_code)]`** in the crate.
6. **Iteration / output limits** — 100k loop iterations, 4 MB output.
7. **Strict CSP** on the page: same-origin only, no inline JS, no remote
   resources.

## Layout

```
pug-wasm/         Rust crate
  src/
    lib.rs        wasm-bindgen entry: compile(source, data_json) → html
    ast.rs        AST node types
    lexer.rs      indentation-aware line splitter
    parser.rs     lines → AST
    expr.rs       restricted-expression parser + evaluator
    emit.rs       AST + data → HTML with auto-escaping
    error.rs      typed errors with line/col
  tests/integration.rs  51 tests including security

web/              Static playground
  index.html      two-pane editor + sandboxed preview
  app.js          loads WASM, debounced re-render
  style.css       minimal styling
  pkg/            wasm-bindgen output (built by build.sh)

build.sh          cargo build + wasm-bindgen → web/pkg/
```
