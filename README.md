# llm-ui-schema-wasm-renderers

A small family of **pure-Rust, security-focused WebAssembly modules** for turning
LLM-friendly schemas and templates into safe HTML — plus a token-efficient data
format for the LLM side of the loop. Every module is written in Rust with
`#![forbid(unsafe_code)]`, ships no embedded JavaScript engine, and is compiled
to a compact `wasm-bindgen` module with a small browser playground.

The shared idea: an LLM (or an app) produces a *small, closed schema* — a JSON UI
spec, a Pug template, or TOON — and these modules render or convert it in the
browser with **no arbitrary-execution primitive**, so untrusted input cannot
escape into the page.

## Sub-projects

| Directory | Pipeline | What it is |
|-----------|----------|------------|
| [`json-render-webassembly`](./json-render-webassembly) | JSON UI spec → **HTML** | A JSON-UI-spec renderer modeled on the [json-render.dev](https://json-render.dev/) generative-UI pattern: an LLM emits a JSON tree of components from a closed catalog, and this renderer turns it into safe HTML with restricted `$state` binding expressions (no `eval`). |
| [`pug-webassembly`](./pug-webassembly) | Pug template → **HTML** | A security-focused Pug renderer. Unlike pug.js (which builds a JS function body and hands it to `new Function(...)` — the SSTI/RCE class behind several Pug CVEs), it evaluates a small host-language expression grammar over plain data, so there is no arbitrary-execution surface. ~233 KB WASM. |
| [`toon-webassembly`](./toon-webassembly) | JSON ⇄ **TOON** (+ combo → HTML) | A bidirectional JSON ⇄ [TOON](https://github.com/toon-format/toon) converter built on the official `toon-format` crate. TOON (Token-Oriented Object Notation) is a YAML/CSV hybrid encoding of the JSON data model that uses ~30–60% fewer LLM tokens than JSON. Includes a `combo` playground that chains TOON → JSON → HTML through the `json-render` module. |

### How they fit together

TOON is the **compact wire format for the LLM output**; `json-render` and `pug`
are the **render targets**. The `toon-webassembly` combo demo shows the full
loop end-to-end:

```
LLM emits TOON  ──toon → json──▶  JSON UI spec  ──json-render──▶  HTML
   (fewer tokens)                                    (safe, no eval)
```

## Layout

Each sub-project is a self-contained Cargo workspace with the same shape:

```
<project>/
  Cargo.toml            # workspace (release profile tuned for small wasm)
  build.sh              # cargo build + wasm-bindgen → web/pkg
  <crate>/src/          # the Rust crate (core logic + optional `wasm` feature)
  <crate>/tests/        # smoke tests
  web/                  # browser playground (index.html + app.js)
  README.md             # project-specific docs
```

## Build

Each project builds independently. You need the Rust `wasm32-unknown-unknown`
target and [`wasm-bindgen-cli`](https://rustwasm.github.io/wasm-bindgen/):

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

cd toon-webassembly   # or json-render-webassembly / pug-webassembly
./build.sh
cd web && python3 -m http.server   # then open the playground
```

See each sub-project's own `README.md` for its API and playground details.

## License

MIT (per sub-crate).
