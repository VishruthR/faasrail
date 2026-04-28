#!/bin/bash
set -e

echo "=== Running ow-loadgen ==="
mkdir -p ~/faasrail/results
cd ~/faasrail/ow-loadgen
./target/release/ow-loadgen \
    --trace ../inputs/diverse_10RPS_bench.csv \
    --outfile ../results/wow_wasmtime.ndjson \
    --no-ramp \
    --concurrency 100

echo ""
echo "=== Collecting metrics ==="
cd ~/faasrail
python3.11 ow-bench-collect.py \
    --ndjson results/wow_wasmtime.ndjson \
    --ow-host http://172.17.0.1:3233 \
    --auth "23bc46b1-71f6-4ed5-8c54-816aa4f8c502:123zO3xZCLrMN6v2BKK1dXYFpXlPkccOFqm12CdAsMgRU4VrNZ9lyGVCGuMDGIwP" \
    --namespace guest \
    --prometheus-url http://172.17.0.1:3233 \
    --out results/wow_results.json \
    --insecure

echo ""
echo "=== Done — results at ~/faasrail/results/wow_results.json ==="