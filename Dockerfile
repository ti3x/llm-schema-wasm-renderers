# syntax=docker/dockerfile:1
#
# Self-contained dev image for all four WASM playgrounds:
#   • Rust toolchain + wasm32 target + a matching wasm-bindgen CLI
#   • every project's WASM compiled into its web/pkg
#   • all source present, ready to rebuild
#   • Python to serve each playground on its own port
FROM rust:1-bookworm

# wasm-bindgen crate and CLI must be the SAME version — pin both here.
ARG WASM_BINDGEN_VERSION=0.2.121

# Python serves the static playgrounds (WASM can't load from file://).
RUN apt-get update \
 && apt-get install -y --no-install-recommends python3 ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# WASM toolchain. Installing the CLI from source is slow but keeps the
# version exactly aligned with the crate pinned below.
RUN rustup target add wasm32-unknown-unknown \
 && cargo install wasm-bindgen-cli --version ${WASM_BINDGEN_VERSION}

WORKDIR /app
COPY . .

# Build each project's WASM into its web/pkg, pinning wasm-bindgen to the
# installed CLI version so the bindgen schema versions match.
RUN set -eux; \
    for d in toon-render-webassembly toon-webassembly json-render-webassembly pug-webassembly; do \
      cd "/app/$d"; \
      cargo update -p wasm-bindgen --precise "${WASM_BINDGEN_VERSION}"; \
      ./build.sh; \
    done

COPY docker/serve-all.sh /usr/local/bin/serve-all.sh
COPY docker/serve.py /usr/local/bin/serve.py
RUN chmod +x /usr/local/bin/serve-all.sh /usr/local/bin/serve.py

# 8000 toon-render · 8001 toon · 8002 json-render · 8003 pug
EXPOSE 8000 8001 8002 8003
CMD ["/usr/local/bin/serve-all.sh"]
