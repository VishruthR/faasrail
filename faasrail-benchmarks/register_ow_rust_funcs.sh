#!/usr/bin/env bash
# Deploy OpenWhisk Rust actions as native binaries
#
# Requires: wsk, jq, bc
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATES_DIR="$SCRIPT_DIR/crates"
BINARIES_DIR="$SCRIPT_DIR/target/x86_64-unknown-linux-musl/release"
RUNS=${RUNS:-5}
OUTPUT="${1:-workloads-rust-code.json}"

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
    "gzip|bench-gzip|bench_gzip|elapsed_ms"
    "aes|bench-aes|bench_aes|elapsed_ms"
    "dp|bench-dp|bench_dp|total_elapsed_ms"
)

deploy_rust_single() {
    local action_name=$1
    local binary_path=$2

    wsk action delete "$action_name" -i >/dev/null 2>&1 || true

    echo $binary_path
    cp $binary_path ./exec
    zip -o archive.zip exec

    local -a create_cmd=(wsk action create "$action_name" --kind native:default "archive.zip" -i)

    "${create_cmd[@]}"

    rm exec
    rm archive.zip
}

for entry in "${BENCHMARKS[@]}"; do
    IFS='|' read -r bench_key crate_dir action_name _elapsed <<< "$entry"

    binary_path="$BINARIES_DIR/$crate_dir"
    if [ ! -f "$binary_path" ]; then
        echo "error: missing $binary_path" >&2
        exit 1
    fi
    deploy_rust_single "$action_name" "$binary_path"
    echo "  deployed: $action_name ($binary_path)"
done
echo ""
