#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_DIR="$SCRIPT_DIR/target/x86_64-unknown-linux-musl/release"
RUNS=${RUNS:-5}
OUTPUT="${1:-workloads.json}"

if ! command -v wsk &>/dev/null; then
    echo "error: wsk not found in PATH" >&2
    exit 1
fi
if ! command -v jq &>/dev/null; then
    echo "error: jq not found in PATH" >&2
    exit 1
fi
if ! command -v zip &>/dev/null; then
    echo "error: zip not found in PATH" >&2
    exit 1
fi

# Build native musl binaries if needed
if [ ! -d "$RELEASE_DIR" ] || [ -z "$(ls "$RELEASE_DIR"/bench-* 2>/dev/null)" ]; then
    echo "Building benchmarks (native musl)..."
    rustup target add x86_64-unknown-linux-musl 2>/dev/null || true
    cargo build --release --target x86_64-unknown-linux-musl \
        --manifest-path "$SCRIPT_DIR/Cargo.toml"
fi

# Each entry: "bench_name|binary_name|action_name|args_for_profiling|elapsed_field"
BENCHMARKS=(
    "float|bench-float|bench_float|10000|elapsed_ms"
    "json|bench-json|bench_json|{\"a\":1,\"b\":[1,2,3]}|elapsed_ms"
    "chameleon|bench-chameleon|bench_chameleon|10 100|elapsed_ms"
    "aes|bench-aes|bench_aes|256 10|elapsed_ms"
    "gzip|bench-gzip|bench_gzip|1|elapsed_ms"
    "disk-seq|bench-disk-seq|bench_disk_seq|4096 1|total_elapsed_ms"
    "disk-rand|bench-disk-rand|bench_disk_rand|4096 1|total_elapsed_ms"
)

# ── Deploy all actions ────────────────────────────────────────────────────────

deploy_action() {
    local action_name=$1
    local bin_name=$2
    local js_main=$3          # Node.js source for index.js

    local tmpdir
    tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" RETURN

    cp "$RELEASE_DIR/$bin_name" "$tmpdir/$bin_name"
    chmod +x "$tmpdir/$bin_name"
    printf '%s\n' "$js_main" > "$tmpdir/index.js"
    (cd "$tmpdir" && zip -q action.zip index.js "$bin_name")
    wsk action delete "$action_name" -i >/dev/null 2>&1 || true
    wsk action create "$action_name" "$tmpdir/action.zip" \
        --kind nodejs:20 -i 2>&1 | tail -1
}

echo "Deploying actions to OpenWhisk..."
for entry in "${BENCHMARKS[@]}"; do
    IFS='|' read -r bench_name bin_name action_name _args _elapsed <<< "$entry"

    if [ ! -f "$RELEASE_DIR/$bin_name" ]; then
        echo "warning: $RELEASE_DIR/$bin_name not found, skipping $bench_name" >&2
        continue
    fi

    case "$bench_name" in
        float)
            js_main="'use strict';
const { execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
function main(params) {
    try {
        const binary = path.join(__dirname, 'bench-float');
        try { fs.chmodSync(binary, 0o755); } catch(e) {}
        const out = execFileSync(binary, [String(params.n || 10000)], { encoding: 'utf8' });
        return JSON.parse(out);
    } catch(e) { return { error: e.message }; }
}
exports.main = main;" ;;
        json)
            js_main="'use strict';
const { execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
function main(params) {
    try {
        const binary = path.join(__dirname, 'bench-json');
        try { fs.chmodSync(binary, 0o755); } catch(e) {}
        const jsonStr = params.json_string || '{\"a\":1,\"b\":[1,2,3]}';
        const out = execFileSync(binary, [jsonStr], { encoding: 'utf8' });
        return JSON.parse(out);
    } catch(e) { return { error: e.message }; }
}
exports.main = main;" ;;
        chameleon)
            js_main="'use strict';
const { execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
function main(params) {
    try {
        const binary = path.join(__dirname, 'bench-chameleon');
        try { fs.chmodSync(binary, 0o755); } catch(e) {}
        const ncol = String(params.num_of_cols || params.ncol || 10);
        const nrow = String(params.num_of_rows || params.nrow || 100);
        const out = execFileSync(binary, [ncol, nrow], { encoding: 'utf8' });
        return JSON.parse(out);
    } catch(e) { return { error: e.message }; }
}
exports.main = main;" ;;
        aes)
            js_main="'use strict';
const { execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
function main(params) {
    try {
        const binary = path.join(__dirname, 'bench-aes');
        try { fs.chmodSync(binary, 0o755); } catch(e) {}
        const msgLen  = String(params.message_length  || 256);
        const numIter = String(params.num_iterations  || 10);
        const out = execFileSync(binary, [msgLen, numIter], { encoding: 'utf8' });
        return JSON.parse(out);
    } catch(e) { return { error: e.message }; }
}
exports.main = main;" ;;
        gzip)
            js_main="'use strict';
const { execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
function main(params) {
    try {
        const binary = path.join(__dirname, 'bench-gzip');
        try { fs.chmodSync(binary, 0o755); } catch(e) {}
        const out = execFileSync(binary, [String(params.file_size || 1)], { encoding: 'utf8' });
        return JSON.parse(out);
    } catch(e) { return { error: e.message }; }
}
exports.main = main;" ;;
        disk-seq)
            js_main="'use strict';
const { execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
function main(params) {
    try {
        const binary = path.join(__dirname, 'bench-disk-seq');
        try { fs.chmodSync(binary, 0o755); } catch(e) {}
        const byteSize = String(params.byte_size || 4096);
        const fileSize = String(params.file_size || 1);
        const out = execFileSync(binary, [byteSize, fileSize], { encoding: 'utf8' });
        return JSON.parse(out);
    } catch(e) { return { error: e.message }; }
}
exports.main = main;" ;;
        disk-rand)
            js_main="'use strict';
const { execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
function main(params) {
    try {
        const binary = path.join(__dirname, 'bench-disk-rand');
        try { fs.chmodSync(binary, 0o755); } catch(e) {}
        const byteSize = String(params.byte_size || 4096);
        const fileSize = String(params.file_size || 1);
        const out = execFileSync(binary, [byteSize, fileSize], { encoding: 'utf8' });
        return JSON.parse(out);
    } catch(e) { return { error: e.message }; }
}
exports.main = main;" ;;
    esac

    deploy_action "$action_name" "$bin_name" "$js_main"
    echo "  deployed: $action_name"
done
echo ""

# ── Profile each action via OpenWhisk ────────────────────────────────────────

results="[]"

for entry in "${BENCHMARKS[@]}"; do
    IFS='|' read -r bench_name bin_name action_name prof_args elapsed_field <<< "$entry"

    # Build the invocation payload (mirrors profile-workloads.sh payload cases)
    case "$bench_name" in
        float)      payload='{"n": 10000000}' ;;                          
        json)       payload='{"json_string": "{\"a\":1,\"b\":[1,2,3]}"}' ;;
        chameleon)  payload='{"num_of_cols": 100, "num_of_rows": 10000}' ;; 
        aes)        payload='{"message_length": 65536, "num_iterations": 500}' ;;  # target ~10 ms
        gzip)       payload='{"file_size": 1}' ;;
        disk-seq)   payload='{"byte_size": 4096, "file_size": 1}' ;;
        disk-rand)  payload='{"byte_size": 4096, "file_size": 1}' ;;
    esac

    echo "Profiling $bench_name via OpenWhisk ($RUNS runs)..."

    sum=0
    sum_sq=0

    payload_file=$(mktemp)
    echo "$payload" > "$payload_file"

    for ((i = 1; i <= RUNS; i++)); do
        raw=$(wsk action invoke "$action_name" --blocking --result \
            --param-file "$payload_file" -i)
        ms=$(echo "$raw" | jq -r ".$elapsed_field")

        sum=$(echo "$sum + $ms" | bc -l)
        sum_sq=$(echo "$sum_sq + $ms * $ms" | bc -l)
        printf "  run %d/%d: %.2f ms\n" "$i" "$RUNS" "$ms"
    done

    rm -f "$payload_file"

    mean=$(echo "scale=6; $sum / $RUNS" | bc -l)

    if [ "$RUNS" -gt 1 ]; then
        variance=$(echo "scale=6; ($sum_sq - $sum * $sum / $RUNS) / ($RUNS - 1)" | bc -l)
        if echo "$variance > 0" | bc -l | grep -q 1; then
            stdev=$(echo "scale=2; sqrt($variance)" | bc -l)
        else
            stdev="0.00"
        fi
    else
        stdev="0.00"
    fi

    # mean_int=$(printf "%.0f" "$mean")

    printf "  => mean=%.2f ms, stdev=%.2f ms\n\n" "$mean" "$stdev"

    results=$(echo "$results" | jq \
        --argjson mean "$mean" \
        --arg stdev "$stdev" \
        --arg bench "$bench_name" \
        --arg payload "$payload" \
        '. + [{
            "mean": $mean,
            "stdev": ($stdev | tonumber),
            "bench": $bench,
            "payload": $payload
        }]')
done

echo "$results" | jq '.' > "$OUTPUT"
echo "Wrote $OUTPUT with $(echo "$results" | jq length) workloads."