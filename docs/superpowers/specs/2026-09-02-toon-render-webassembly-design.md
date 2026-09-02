# toon-render-webassembly — Design

**Date:** 2026-09-02
**Status:** Approved (design), pending implementation plan

## Goal

Add a fourth sub-project, `toon-render-webassembly`, that turns **TOON** (an
LLM's token-efficient output) into **HTML** in a *single* WebAssembly module —
`TOON → JSON UI spec → HTML`.

Today this pipeline only exists as a browser-side demo (`toon-webassembly/web/combo.js`)
that loads **two** WASM modules (`toon_wasm`, `json_render_wasm`) and stitches
them together with a JavaScript adapter. This project collapses that into one
Rust crate / one WASM binary.

## Non-goals

- Reimplementing TOON parsing or the HTML renderer. Both already exist as
  crates and MUST be reused.
- Changing `toon-webassembly` or `json-render-webassembly`. They stay as-is;
  this crate depends on them.

## Constraints

- **Reuse, don't rewrite.** Depend on `toon-wasm` and `json-render-wasm` via
  path dependencies. The only genuinely new logic is the spec adapter.
- Match the repo conventions: own Cargo workspace, tuned release profile,
  `build.sh` producing `web/pkg`, a `web/` playground, a `README.md`, and
  `#![forbid(unsafe_code)]`.

## Architecture

```
toon-render-webassembly/
  Cargo.toml                # [workspace] members = ["toon-render-wasm"]
                            # release profile: opt-level="z", lto, codegen-units=1,
                            #                  panic="abort", strip
  build.sh                  # cargo build --release --target wasm32-unknown-unknown
                            #   --features wasm -p toon-render-wasm
                            # wasm-bindgen --target web --out-name toon_render_wasm
  toon-render-wasm/
    Cargo.toml
    src/
      lib.rs                # core pipeline + #[cfg(feature="wasm")] bindings
      adapt.rs             # json-render.dev → simple-form adapter (ported)
    tests/
      smoke.rs
  web/
    index.html
    app.js
    styles.css
  README.md
```

### Dependencies

```toml
# toon-render-wasm/Cargo.toml
[dependencies]
toon-wasm        = { path = "../../toon-webassembly/toon-wasm" }
json-render-wasm = { path = "../../json-render-webassembly/json-render-wasm" }
serde_json = "1"
wasm-bindgen = { version = "0.2", optional = true }

[features]
default = []
wasm = ["wasm-bindgen"]
```

Both dependency crates expose their core logic as `rlib` without needing their
own `wasm` feature:
- `toon_wasm::toon_to_json(&str, &ConvOptions) -> ConvResult<String>`
- `json_render_wasm::render(spec_json: &str, state_json: &str) -> RenderResult<String>`

## Data flow

```
TOON text
  │  ① toon_wasm::toon_to_json(toon, &ConvOptions::default())   [reused]
  ▼
JSON spec string  ──parse──▶  serde_json::Value
  │  ② adapt::adapt(value)                                       [NEW — the only new logic]
  │      • isIndirected? (has string `root` + object `elements`)
  │        - no  → passthrough (already simple {tag,props,children} form)
  │        - yes → resolve $state / $template bindings against embedded `state`,
  │                walk from `root` through `elements`, map each component `type`
  │                to a catalog tag, synthesizing non-catalog components from
  │                primitives.
  ▼
simple {tag,props,children} Value  ──to_string──▶  spec string
  │  ③ json_render_wasm::render(spec, "")                        [reused]
  ▼
HTML
```

Bindings are resolved inside the adapter, so step ③ passes empty state
(`""`) — identical to `combo.js`'s `compile(simpleJson, "")`.

## The adapter (`adapt.rs`) — ported from `combo.js:184-372`

Operates on `serde_json::Value`. Functions:

- `is_indirected(spec) -> bool` — object with string `root` and object `elements`.
- `json_pointer(root, path) -> Value` + `decode_pointer(seg)` — RFC-6901-ish
  pointer resolution (`~1`→`/`, `~0`→`~`).
- `resolve_bindings(value, state) -> Value` — recursive:
  - `{ "$state": "/ptr" }` → the value at that pointer in `state`.
  - `{ "$template": "…${/ptr}…" }` → string interpolation of pointers.
  - otherwise recurse into arrays/objects.
- helpers: `sparkline(values)`, `progress_bar(pct, width)`, `level_from_string(s)`.
- `adapt(spec) -> Value` with inner `build_by_id(id)` / `build_node(node)`:
  a `match` on `node.type` producing simple-form nodes. Mapping (from the JS):

  | `type`                         | Output |
  |--------------------------------|--------|
  | `Card`                         | `Card` (keeps `title` if present) + children |
  | `Container`, `Stack`           | `Container` + children |
  | `Heading`                      | `Heading { level, value }` (`level_from_string`) |
  | `Text`                         | `Text { value }` |
  | `Button`                       | `Button { label, variant }` |
  | `Image`                        | `Container` with alt-text heading + dimensions (CSP blocks remote `<img>`) |
  | `Separator`                    | `Container` with a rule `Text` |
  | `Metric`                       | `Container`: label `Text`, big `Heading`, change `Text` |
  | `LineGraph`                    | `Container`: title + `sparkline` `Text` + summary `Text` |
  | `Progress`                     | `Container`: label `Text` + `progress_bar` `Text` |
  | *(unknown)*                    | `Container`: `[type]` heading + recursively-built children (placeholder) |

  Missing element id → `Text { value: "[missing element: <id>]" }`.

All output tags (`Text`, `Heading`, `Container`, `Card`, `Button`) are in the
renderer's closed catalog (`json-render-wasm/src/catalog.rs`), and
children-bearing tags (`Container`, `Card`) accept children — verified.

## Public / core API

```rust
// core (feature-independent)
pub fn render(toon_spec: &str, state_json: &str) -> Result<String, Error>;
// pipeline: toon_to_json → adapt → json_render_wasm::render
```

`Error` wraps the two upstream error types plus a JSON-parse variant
(via `thiserror`, matching sibling crates).

### WASM bindings (`#[cfg(feature = "wasm")]`) — mirror json-render-wasm

```rust
#[wasm_bindgen]
pub fn compile(toon_spec: &str, state_json: &str) -> Result<String, JsError>;

#[wasm_bindgen]
pub struct Template { /* parsed simple-form spec */ }
#[wasm_bindgen]
impl Template {
    #[wasm_bindgen(constructor)]
    pub fn new(toon_spec: &str) -> Result<Template, JsError>; // toon→json→adapt once
    pub fn render(&self, state_json: &str) -> Result<String, JsError>;
}
```

Note: because the adapter resolves `$state` bindings against the spec's *embedded*
`state` at parse time, `Template` captures the adapted simple-form spec; its
`render(state)` forwards `state` to the renderer for any residual simple-form
`$state` bindings the native renderer itself supports. `compile` is the primary
path and matches the combo demo.

## Error handling

- Invalid TOON → error from `toon_to_json`, surfaced as `JsError`.
- Invalid/again-invalid JSON → JSON-parse error variant.
- Spec rejected by the renderer's catalog → `RenderError`, surfaced as `JsError`.
- Unknown json-render.dev component `type` → non-fatal placeholder (page still
  renders), matching `combo.js`.

## Testing (`tests/smoke.rs`)

1. **Native form:** TOON encoding of `{tag:"Card",props:{title:...},children:[Text]}`
   → HTML contains the title and text.
2. **json-render.dev form:** TOON with `{root,elements,state}` + a `$state` and a
   `$template` binding → HTML reflects resolved values.
3. **Unknown component:** `type: "Widget"` → placeholder `[Widget]` heading, no error.
4. **Round-trip sanity:** a spec run through `compile` equals `render` core fn.

## Web playground (`web/`)

Single-module demo: one `<textarea>` for TOON, live-updating panes for the
generated simple-form JSON (debugging aid) and the rendered HTML, plus a preview
iframe. Imports **only** `./pkg/toon_render_wasm.js` — no second module, no JS
adapter. Reuses the styling approach of the existing playgrounds. A couple of
presets (native + .dev form) seed the editor.

## Repo README update

Add a fourth row to the top-level `README.md` table describing
`toon-render-webassembly` (single-module TOON→HTML), and note it supersedes the
two-module `combo` demo in `toon-webassembly`.
