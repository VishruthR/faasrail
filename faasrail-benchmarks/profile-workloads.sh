#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_DIR="$SCRIPT_DIR/target/wasm32-wasip1/release"
RUNS=${RUNS:-5}
OUTPUT="${1:-workloads.json}"

if ! command -v wasmtime &>/dev/null; then
    echo "error: wasmtime not found in PATH" >&2
    exit 1
fi
if ! command -v jq &>/dev/null; then
    echo "error: jq not found in PATH" >&2
    exit 1
fi

# Build if needed
if [ ! -d "$RELEASE_DIR" ] || [ -z "$(ls "$RELEASE_DIR"/*.wasm 2>/dev/null)" ]; then
    echo "Building benchmarks..."
    make -C "$SCRIPT_DIR" build
fi

# Each entry: "bench_name|wasm_binary|wasmtime_extra_flags|args...|elapsed_field"
# The last element is the JSON field that holds the elapsed time.
BENCHMARKS=(
    "float|bench-float||10000|elapsed_ms"
    "json|bench-json||{\"a\":1,\"b\":[1,2,3]}|elapsed_ms"
    "chameleon|bench-chameleon||10 100|elapsed_ms"
    "aes|bench-aes||256 10|elapsed_ms"
    "gzip|bench-gzip|--dir=/tmp|1|elapsed_ms"
    "disk-seq|bench-disk-seq|--dir=/tmp|4096 1|total_elapsed_ms"
    "disk-rand|bench-disk-rand|--dir=/tmp|4096 1|total_elapsed_ms"
)

results="[]"

for entry in "${BENCHMARKS[@]}"; do
    IFS='|' read -r bench_name wasm_name extra_flags args elapsed_field <<< "$entry"
    wasm="$RELEASE_DIR/${wasm_name}.wasm"

    if [ ! -f "$wasm" ]; then
        echo "warning: $wasm not found, skipping $bench_name" >&2
        continue
    fi

    echo "Profiling $bench_name ($RUNS runs)..."

    sum=0
    sum_sq=0

    for ((i = 1; i <= RUNS; i++)); do
        # shellcheck disable=SC2086
        raw=$(wasmtime run $extra_flags "$wasm" $args)
        ms=$(echo "$raw" | jq -r ".$elapsed_field")

        sum=$(echo "$sum + $ms" | bc -l)
        sum_sq=$(echo "$sum_sq + $ms * $ms" | bc -l)
        printf "  run %d/%d: %.2f ms\n" "$i" "$RUNS" "$ms"
    done

    mean=$(echo "scale=2; $sum / $RUNS" | bc -l)

    if [ "$RUNS" -gt 1 ]; then
        variance=$(echo "scale=6; ($sum_sq - $sum * $sum / $RUNS) / ($RUNS - 1)" | bc -l)
        # bc sqrt via Newton's method; clamp negative variance from fp noise to 0
        if echo "$variance > 0" | bc -l | grep -q 1; then
            stdev=$(echo "scale=2; sqrt($variance)" | bc -l)
        else
            stdev="0.00"
        fi
    else
        stdev="0.00"
    fi

    # Round mean to integer for shrinkray (it expects dur_ms as int)
    mean_int=$(printf "%.0f" "$mean")

    # Build the payload string: a JSON object of the arguments used.
    # This mirrors how shrinkray expects "payload" to be a JSON-encoded string.
    case "$bench_name" in
        float)
            payload='{"n": 10000}' ;;
        json)
            payload='{"json_string": "{\"a\":1,\"b\":[1,2,3]}"}' ;;
        chameleon)
            payload='{"num_of_cols": 10, "num_of_rows": 100}' ;;
        aes)
            payload='{"message_length": 256, "num_iterations": 10}' ;;
        gzip)
            payload='{"file_size": 1}' ;;
        disk-seq)
            payload='{"byte_size": 4096, "file_size": 1}' ;;
        disk-rand)
            payload='{"byte_size": 4096, "file_size": 1}' ;;
    esac

    printf "  => mean=%.2f ms, stdev=%.2f ms\n\n" "$mean" "$stdev"

    results=$(echo "$results" | jq \
        --argjson mean "$mean_int" \
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
