# json-render-webassembly

A pure-Rust **JSON-UI-spec → HTML** renderer compiled to WebAssembly. Modeled on
the [json-render.dev](https://json-render.dev/) generative-UI pattern from
Vercel: an LLM produces a small JSON tree of components from a closed catalog,
and this renderer turns that tree into safe HTML.

Sibling project: [`../pug-webassembly`](../pug-webassembly) — same architecture,
same security posture, but for Pug templates.

## What it does (and does not)

The crate exposes two WASM entry points:

```js
import init, { Template, compile } from "./pkg/json_render_wasm.js";
await init();

// One-shot: parse + render in one go.
const html = compile(specJson, stateJson);

// Parse once, render many times with different state.
const tmpl = new Template(specJson);
const html = tmpl.render(stateJson);
tmpl.free(); // wasm-bindgen does not GC for you
```

A spec is a JSON tree of nodes from a fixed catalog (10 components):
`Text`, `Heading`, `Link`, `Image`, `Container`, `Card`, `List`, `Table`,
`Button`, `Input`.

```json
{
  "tag": "Card",
  "props": { "title": "Users" },
  "children": [
    {
      "tag": "List",
      "props": { "items": "$bindState.users" },
      "children": [
        { "tag": "Text", "props": { "value": "$item.name" } }
      ]
    }
  ]
}
```

Render that against `{"users":[{"name":"Ada"},{"name":"Bob"}]}` and you get a
`<ul>` with two `<li>` rows.

### What's intentionally not supported

- Arbitrary HTML tags. Only the 10 catalog components.
- Inline JS / event handlers. There is no `onClick`, no expression language
  beyond paths.
- Function calls or operators in bindings. Bindings are pure path lookups
  — `$bindState.x.y`, `$item.x`, `$index`.
- Two-way data binding. State is read-only during a render. Mutation is the
  embedding app's job.

## Security model

The threat model assumes a hostile spec author (often: an LLM whose output
you do not fully trust). The renderer answers with a defense-in-depth stack:

1. **Closed catalog.** Unknown `tag` values fail at parse time. There is no
   way to address `<script>`, `<iframe>`, or `<style>` directly.
2. **Closed prop set per component.** Unknown spec keys (`onClick`, `style`,
   …) are rejected.
3. **Restricted bindings.** Only `$bindState.<path>` / `$item.<path>` / `$index`
   are accepted. Strings containing parentheses, operators, or JS keywords
   (`eval`, `function`, `__proto__`, …) are rejected at parse time.
4. **Auto-escape everywhere.** All resolved values pass through HTML/attribute
   escaping. Output never contains a raw value.
5. **URL sanitization.** `href` and `src` props starting with `javascript:`,
   `data:`, or `vbscript:` are replaced with `#`.
6. **Output + iteration limits.** Hostile specs cannot blow up output beyond
   4 MB or run more than 100k `List` / `Table` row iterations.
7. **Sandboxed preview.** The `web/` playground renders into an `<iframe sandbox>`
   with a strict CSP. `<script>` and event handlers are stripped in JS as
   defense-in-depth even though the renderer never emits them.

## Build

```bash
./build.sh
```

This runs `cargo build --release --target wasm32-unknown-unknown --features wasm`
and stages the output through `wasm-bindgen` into `web/pkg/`.

## Tests

```bash
cargo test
```

Covers parsing, binding resolution, list iteration, escaping, URL
sanitization, and catalog enforcement.
