#!/usr/bin/env bash
set -euo pipefail

# Build a minimal TypeScript site, serve it, and execute its generated module
# in headless Firefox. This intentionally uses the TypeScript Cargo feature.
root=$(mktemp -d "${TMPDIR:-/tmp}/taxus-typescript-browser.XXXXXX")
server_pid=""
cleanup() {
    if [[ -n "$server_pid" ]]; then
        kill "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    rm -rf "$root"
}
trap cleanup EXIT

cargo run -p taxus-cli --features typescript -- init "$root/site" \
    --name "TypeScript Browser Smoke" --no-islands --typescript >/dev/null

cat > "$root/site/scripts/main.ts" <<'TS'
const marker: string = "passed";
document.documentElement.dataset.typescriptSmoke = marker;
TS

cargo run -p taxus-cli --features typescript -- build \
    --dir "$root/site" --output "$root/site/dist" >/dev/null

python3 -m http.server 4173 --bind 127.0.0.1 --directory "$root/site/dist" \
    >"$root/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 30); do
    if curl --silent --fail http://127.0.0.1:4173/ >/dev/null; then
        break
    fi
    sleep 0.2
done

python3 scripts/typescript-browser-smoke.py --url http://127.0.0.1:4173/
