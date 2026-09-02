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

## Run the playgrounds (Docker)

A `Dockerfile` + `Makefile` package the whole toolchain, build every project's
WASM, and serve all four playgrounds at once — each on its own port. You only
need Docker installed.

```bash
make build        # build the image: deps + all WASM compiled (first run is slow)
make playground   # launch all four playgrounds; Ctrl-C to stop
```

Then open, each in a browser:

| Port | Project | What you'll see |
|------|---------|-----------------|
| **[8000](http://localhost:8000/)** | `toon-render-webassembly` | Edit TOON → live HTML + preview, via one WASM module. Shows the JSON decoded from TOON under the editor with the TOON⇄JSON size diff. Presets for the flat and json-render.dev forms. |
| **[8001](http://localhost:8001/)** | `toon-webassembly` | JSON ⇄ TOON converter with token stats. Also [`/combo.html`](http://localhost:8001/combo.html) — the original two-module TOON → JSON → HTML demo. |
| **[8002](http://localhost:8002/)** | `json-render-webassembly` | Edit a JSON UI spec + state → live HTML, with two savings comparisons (spec-only vs spec+state) against the generated HTML. |
| **[8003](http://localhost:8003/)** | `pug-webassembly` | Edit a Pug template → live HTML, with two savings comparisons (template-only vs template+data). Also [`/combo.html`](http://localhost:8003/combo.html). |

Other targets: `make stop` (remove the running container), `make clean` (also
remove the image), `make help` (list targets).

> The servers send `Cache-Control: no-store`, so after a `make build` a normal
> browser refresh always picks up the rebuilt assets.

<details>
<summary>Run a single playground without Docker</summary>

Each sub-project also has a `build.sh` that stages its WASM into `web/pkg/`.
Build it, then serve `web/` over HTTP (WASM can't load from a `file://` URL):

```bash
cd toon-render-webassembly     # or any other sub-project
./build.sh                     # cargo build + wasm-bindgen → web/pkg
cd web && python3 -m http.server 8000
```

Then open <http://localhost:8000/>.
</details>

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
