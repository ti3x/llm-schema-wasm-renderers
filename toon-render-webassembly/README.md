# toon → html · WebAssembly

A **single** WebAssembly module that turns **TOON** (an LLM's token-efficient
output) into safe **HTML**:

```
TOON  ──▶  JSON UI spec  ──▶  (adapt json-render.dev form)  ──▶  HTML
```

This collapses into one Rust crate / one `.wasm` binary what the
[`toon-webassembly`](../toon-webassembly) `combo` demo does with **two** WASM
modules glued together by JavaScript. It is pure glue — the heavy lifting is
reused, not reimplemented:

- [`toon-wasm`](../toon-webassembly/toon-wasm) — TOON → JSON (path dependency)
- [`json-render-wasm`](../json-render-webassembly/json-render-wasm) — JSON UI spec → HTML (path dependency)

The only new code is [`src/adapt.rs`](./toon-render-wasm/src/adapt.rs), a Rust
port of the JS adapter from the combo demo. It converts the
[json-render.dev](https://json-render.dev/) indirected spec form
(`{ root, elements, state }` with `$state` / `$template` bindings) into the
renderer's flat `{ tag, props, children }` form, resolving bindings against the
embedded `state` and synthesizing non-catalog components (Metric, LineGraph,
Progress, Separator, Image) from primitives.

## API

```js
import init, { compile, Template } from "./pkg/toon_render_wasm.js";
await init();

// One-shot: TOON spec + state → HTML.
const html = compile(toonSpec, "");

// Parse + adapt once, render many times.
const tmpl = new Template(toonSpec);
const html = tmpl.render(stateJson);
tmpl.free(); // wasm-bindgen does not GC for you
```

Both spec forms are accepted:

- **Flat form** — TOON that decodes to `{ tag, props, children }` is rendered
  as-is (native `$bindState` bindings resolved against the `state` argument).
- **json-render.dev form** — TOON that decodes to `{ root, elements, state }`
  is adapted first; its `$state` / `$template` bindings are resolved against the
  embedded `state`, so `compile(toon, "")` is the usual call.

## Build

Requires the wasm target and `wasm-bindgen-cli`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli   # must match the wasm-bindgen crate version

./build.sh                       # → web/pkg/toon_render_wasm{.js,_bg.wasm}
cd web && python3 -m http.server # open the playground
```

> **Version note:** `wasm-bindgen` (crate) and `wasm-bindgen-cli` must be the
> same version. If `build.sh` reports a schema mismatch, align them, e.g.
> `cargo update -p wasm-bindgen --precise <cli-version>`.

## Test

```bash
cargo test   # end-to-end: TOON → HTML for flat form, .dev form, bindings, synthesis
```

## Playground

`web/` is a single-module demo: edit TOON on the left, see the generated HTML
and a live preview. It imports **only** `toon_render_wasm` — no second module,
no JS adapter. Two presets show the flat form and the json-render.dev form with
`$state`/`$template` bindings.

## License

MIT
