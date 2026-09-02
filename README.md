# llm-schema-wasm-renderers

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
| [`json-render-webassembly`](./json-render-webassembly) | JSON UI spec → **HTML** | A JSON-UI-spec renderer modeled on the [json-render.dev](https://json-render.dev/) generative-UI pattern: an LLM emits a JSON tree of components from a closed catalog, rendered to safe HTML with restricted `$bindState` binding expressions (no `eval`). |
| [`pug-webassembly`](./pug-webassembly) | Pug template → **HTML** | A security-focused Pug renderer. Unlike pug.js (which builds a JS function body and hands it to `new Function(...)` — the SSTI/RCE class behind several Pug CVEs), it evaluates a small host-language expression grammar over plain data, so there is no arbitrary-execution surface. ~233 KB WASM. |
| [`toon-webassembly`](./toon-webassembly) | JSON ⇄ **TOON** (+ combo → HTML) | A bidirectional JSON ⇄ [TOON](https://github.com/toon-format/toon) converter built on the official `toon-format` crate. TOON (Token-Oriented Object Notation) is a YAML/CSV hybrid that uses ~30–60% fewer LLM tokens than JSON. Includes a two-module `combo` playground that chains TOON → JSON → HTML. |
| [`toon-render-webassembly`](./toon-render-webassembly) | TOON → JSON spec → **HTML** | A **single** WASM module that does the whole TOON→HTML pipeline, reusing `toon-wasm` and `json-render-wasm` plus a Rust port of the combo's spec adapter. Supersedes the two-module + JS-glue `combo` demo. |

### How they fit together

TOON is the **compact wire format for the LLM output**; `json-render` and `pug`
are the **render targets**. `toon-render-webassembly` fuses the TOON and
`json-render` halves into one binary:

```
LLM emits TOON  ──toon → json──▶  JSON UI spec  ──json-render──▶  HTML
   (fewer tokens)                                    (safe, no eval)
```

## Prerequisites

To build any project you need the Rust `wasm32-unknown-unknown` target and
[`wasm-bindgen-cli`](https://rustwasm.github.io/wasm-bindgen/):

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

> The `wasm-bindgen` crate and the `wasm-bindgen-cli` must be the **same
> version**. If a build reports a schema-version mismatch, align them, e.g.
> `cargo update -p wasm-bindgen --precise <cli-version>` in that project.

## Run a playground

Every sub-project has a `build.sh` that compiles the crate and stages the WASM
into its own `web/pkg/`, and a `web/` folder with the playground. The pattern is
identical for all four — build, then serve `web/` over HTTP (WASM modules can't
be loaded from a `file://` URL):

```bash
cd toon-render-webassembly     # or any other sub-project
./build.sh                     # cargo build + wasm-bindgen → web/pkg
cd web
python3 -m http.server 8000    # any static server works
```

Then open **http://localhost:8000/** in a browser.

Playground pages by project:

| Project | URL to open | What you'll see |
|---------|-------------|-----------------|
| `toon-render-webassembly` | `/` | Edit TOON → live HTML + preview, via one WASM module. Presets for the flat and json-render.dev forms. |
| `toon-webassembly` | `/` | JSON ⇄ TOON converter. Also `/combo.html` — the original two-module TOON → JSON → HTML demo. |
| `json-render-webassembly` | `/` | Edit a JSON UI spec + state → live HTML. |
| `pug-webassembly` | `/` | Edit a Pug template → live HTML. |

Stop the server with `Ctrl-C`. To try another project, `cd` into it, run its
`build.sh`, and serve its `web/` folder the same way.

## Test

Each project has Rust smoke tests:

```bash
cd <project> && cargo test
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

See each sub-project's own `README.md` for its API and details.

## License

MIT (per sub-crate).
