#!/bin/bash
set -e

echo "=== Running ow-loadgen (distributed WOW) ==="
mkdir -p ~/faasrail/results
cd ~/faasrail/ow-loadgen
./target/release/ow-loadgen \
    --trace ../inputs/diverse_10RPS_bench.csv \
    --outfile ../results/wow_distributed_wasmtime.ndjson \
    --no-ramp \
    --concurrency 100 \
    --ow-host https://sp26-cs525-1809.cs.illinois.edu:10001 \
    --auth "23bc46b1-71f6-4ed5-8c54-816aa4f8c502:123zO3xZCLrMN6v2BKK1dXYFpXlPkccOFqm12CdAsMgRU4VrNZ9lyGVCGuMDGIwP" \
    --insecure

echo ""
echo "=== Collecting metrics ==="
cd ~/faasrail
python3.11 ow-bench-collect.py \
    --ndjson results/wow_distributed_wasmtime.ndjson \
    --ow-host https://sp26-cs525-1809.cs.illinois.edu:10001 \
    --auth "23bc46b1-71f6-4ed5-8c54-816aa4f8c502:123zO3xZCLrMN6v2BKK1dXYFpXlPkccOFqm12CdAsMgRU4VrNZ9lyGVCGuMDGIwP" \
    --namespace guest \
    --prometheus-url http://172.17.0.1:9090 \
    --out results/wow_distributed_results.json \
    --insecure

echo ""
echo "=== Done — results at ~/faasrail/results/wow_distributed_results.json ==="