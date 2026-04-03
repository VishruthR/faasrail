#!/usr/bin/env bash
# Deploy faasrail benchmarks as single-file OpenWhisk Rust actions (rust:1.34): each
# action is crates/<name>/src/lib.rs. The runtime supplies serde / serde_json /
# serde_derive (no extra crates.io deps in the action source).
#
# Requires: wsk, jq, bc
#
# If your controller expects the upstream Docker image instead of --kind rust:1.34:
#   export OW_RUST_DOCKER=openwhisk/action-rust-v1.34

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATES_DIR="$SCRIPT_DIR/crates"
RUNS=${RUNS:-5}
OUTPUT="${1:-workloads-rust-code.json}"

RUST_KIND=${RUST_KIND:-rust:1.34}
OW_RUST_DOCKER=${OW_RUST_DOCKER:-}

if ! command -v wsk &>/dev/null; then
    echo "error: wsk not found in PATH" >&2
    exit 1
fi
if ! command -v jq &>/dev/null; then
    echo "error: jq not found in PATH" >&2
    exit 1
fi
if ! command -v bc &>/dev/null; then
    echo "error: bc not found in PATH" >&2
    exit 1
fi

# bench_key|crate_dir|action_name|elapsed_field (field in --result JSON)
BENCHMARKS=(
    "float|bench-float|bench_float|elapsed_ms"
    "json|bench-json|bench_json|elapsed_ms"
    "chameleon|bench-chameleon|bench_chameleon|elapsed_ms"
    "disk-seq|bench-disk-seq|bench_disk_seq|total_elapsed_ms"
    "disk-rand|bench-disk-rand|bench_disk_rand|total_elapsed_ms"
)

deploy_rust_single() {
    local action_name=$1
    local rs_path=$2

    wsk action delete "$action_name" -i >/dev/null 2>&1 || true

    local -a create_cmd=(wsk action create "$action_name" "$rs_path" -i)
    if [ -n "${OW_RUST_DOCKER}" ]; then
        create_cmd+=(--docker "$OW_RUST_DOCKER")
    else
        create_cmd+=(--kind "$RUST_KIND")
    fi

    "${create_cmd[@]}" 2>&1 | tail -1
}

if [ -n "${OW_RUST_DOCKER}" ]; then
    echo "Deploying Rust actions (docker image: ${OW_RUST_DOCKER})..."
else
    echo "Deploying Rust actions (single .rs, kind: ${RUST_KIND})..."
fi
for entry in "${BENCHMARKS[@]}"; do
    IFS='|' read -r bench_key crate_dir action_name _elapsed <<< "$entry"

    rs_path="$CRATES_DIR/$crate_dir/src/lib.rs"
    if [ ! -f "$rs_path" ]; then
        echo "error: missing $rs_path" >&2
        exit 1
    fi
    deploy_rust_single "$action_name" "$rs_path"
    echo "  deployed: $action_name ($rs_path)"
done
echo ""

# ── Profile each action via OpenWhisk ─────────────────────────────────────────

results="[]"

for entry in "${BENCHMARKS[@]}"; do
    IFS='|' read -r bench_key crate_dir action_name elapsed_field <<< "$entry"

    case "$bench_key" in
        float)      payload='{"n": 10000}' ;;
        json)       payload='{"json_string": "{\"a\":1,\"b\":[1,2,3]}"}' ;;
        chameleon)  payload='{"num_of_cols": 100, "num_of_rows": 100}' ;;
        disk-seq)   payload='{"byte_size": 4096, "file_size": 1}' ;;
        disk-rand)  payload='{"byte_size": 4096, "file_size": 1}' ;;
        *)
            echo "error: unknown bench_key $bench_key" >&2
            exit 1
            ;;
    esac

    echo "Profiling $bench_key via OpenWhisk ($RUNS runs)..."

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

    printf "  => mean=%.2f ms, stdev=%.2f ms\n\n" "$mean" "$stdev"

    results=$(echo "$results" | jq \
        --argjson mean "$mean" \
        --arg stdev "$stdev" \
        --arg bench "$bench_key" \
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
