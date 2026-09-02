#!/usr/bin/env python3
"""Static file server that disables caching, so rebuilt playground assets
are always re-fetched (avoids stale app.js / index.html mismatches)."""
import sys
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer


class NoCacheHandler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-store, must-revalidate")
        self.send_header("Pragma", "no-cache")
        self.send_header("Expires", "0")
        super().end_headers()


def main():
    port = int(sys.argv[1])
    directory = sys.argv[2]
    handler = partial(NoCacheHandler, directory=directory)
    ThreadingHTTPServer(("", port), handler).serve_forever()


if __name__ == "__main__":
    main()
