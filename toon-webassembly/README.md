# toon-webassembly

Bidirectional **JSON ⇄ TOON** converter, compiled to WebAssembly. Built on
top of the official [`toon-format`](https://crates.io/crates/toon-format)
crate (v0.4) and shipped as a tiny `wasm-bindgen` shim plus a live
two-pane playground.

[TOON](https://github.com/toon-format/toon) (Token-Oriented Object Notation)
is a YAML/CSV-hybrid encoding of the JSON data model designed to use 30–60%
fewer LLM tokens than JSON. This project is a sibling of:

- [`../pug-webassembly`](../pug-webassembly) — Pug templates → HTML
- [`../json-render-webassembly`](../json-render-webassembly) — JSON UI spec → HTML

## API

```js
import init, { json_to_toon, toon_to_json } from "./pkg/toon_wasm.js";
await init();

const toon = json_to_toon(jsonString, optionsJson);
const json = toon_to_json(toonString, optionsJson);
```

`optionsJson` is an empty string (for defaults) or a JSON object with any
subset of:

```json
{
  "delimiter": "comma" | "tab" | "pipe",
  "indent": 2,
  "strict": true,
  "coerceTypes": true,
  "keyFolding": false,
  "expandPaths": false
}
```

Defaults match the TOON v3.0 spec: comma delimiter, 2-space indent, strict
decoding with type coercion, no key folding / path expansion.

## Example

JSON:
```json
{ "users": [
    { "id": 1, "name": "Ada",   "role": "admin" },
    { "id": 2, "name": "Bob",   "role": "user"  },
    { "id": 3, "name": "Grace", "role": "user"  }
] }
```

TOON (default options):
```
users[3]{id,name,role}:
  1,Ada,admin
  2,Bob,user
  3,Grace,user
```

Typical savings: ~40% fewer characters (≈ tokens) on this shape.

## Build

```bash
./build.sh
```

Produces `web/pkg/toon_wasm.wasm` + `.js` (plus `.d.ts` files for editor
tooling). Serve the `web/` directory with any static server:

```bash
python3 -m http.server -d web 8000
```

## Tests

```bash
cargo test
```

Covers round-trip for all the major JSON shapes (objects, nested objects,
tabular arrays, mixed arrays, nulls/floats), delimiter switching,
strict-mode rejection, and error-variant mapping.

## Design notes

- **Engine.** Uses `toon-format` for both directions rather than
  re-implementing the format. TOON has 358 spec fixtures and a context-
  sensitive grammar — reusing the spec-aligned crate is the right call
  for a playground.
- **Default features disabled.** The `toon-format` crate's default
  `cli` feature pulls in `clap`, `ratatui`, `tiktoken-rs`, `syntect`,
  etc. — we set `default-features = false` to keep the WASM lean.
- **Token approximation.** The playground shows `ceil(chars/4)` as a
  cheap token estimate rather than bundling `tiktoken-rs`. The relative
  savings are accurate; absolute counts are ~within ±10%.
- **Bidirectional auto-detection.** The pane the user last typed in
  drives the conversion; a `silent` flag prevents the regenerated output
  from re-triggering the reverse direction.
