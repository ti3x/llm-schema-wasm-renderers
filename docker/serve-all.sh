#!/usr/bin/env bash
# Serve every playground's web/ directory on its own port.
set -eu

SERVE=/usr/local/bin/serve.py
python3 "$SERVE" 8000 /app/toon-render-webassembly/web &
python3 "$SERVE" 8001 /app/toon-webassembly/web &
python3 "$SERVE" 8002 /app/json-render-webassembly/web &
python3 "$SERVE" 8003 /app/pug-webassembly/web &

echo
echo "Playgrounds are live:"
echo "  toon-render-webassembly  ->  http://localhost:8000/"
echo "  toon-webassembly         ->  http://localhost:8001/   (also /combo.html)"
echo "  json-render-webassembly  ->  http://localhost:8002/"
echo "  pug-webassembly          ->  http://localhost:8003/   (also /combo.html)"
echo
echo "Press Ctrl-C to stop."

# Exit (and take the container down) if any server dies.
wait -n
