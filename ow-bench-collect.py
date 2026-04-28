"""
ow-bench-collect.py
Accumulates cold start time, server-side duration, memory and computes percentile
latency stats. Optionally scrapes a Prometheus endpoint for system metrics.


Usage:
   python ow-bench-collect.py \
       --ndjson results.ndjson \
       --ow-host https://HOST_URL \
       --auth "user:pass" \
       --namespace guest \
       --prometheus-url http://invoker-host:9095 \
       --out benchmark_results.json \
       --insecure
"""

import argparse
import json
import statistics
import time
from pathlib import Path

import requests
import urllib3

def percentile(sorted_values: list, p: float) -> float:
    if not sorted_values:
        return 0.0
    k = (len(sorted_values) - 1) * p / 100
    lo, hi = int(k), min(int(k) + 1, len(sorted_values) - 1)
    return sorted_values[lo] + (sorted_values[hi] - sorted_values[lo]) * (k - lo)

def fetch_activation(session, ow_host, namespace, activation_id):
    url = f"{ow_host}/api/v1/namespaces/{namespace}/activations/{activation_id}"
    try:
        resp = session.get(url, timeout=10)
        resp.raise_for_status()
        return resp.json()
    except requests.RequestException as e:
        print(f"  WARN: could not fetch activation {activation_id}: {e}")
        return None

def extract_annotation(annotations, key):
    for ann in annotations:
        if ann.get("key") == key:
            return ann.get("value")
    return None

PROMETHEUS_METRICS_OF_INTEREST = [
    "gauge_containerPool_activeCount_counter",
    "gauge_containerPool_idlesCount_counter",
    "gauge_containerPool_prewarmCount_counter",
    "gauge_containerPool_activeSize_counter_bytes",
    "gauge_containerPool_idlesSize_counter_bytes",
    "gauge_containerPool_runBufferCount_counter",
    "counter_invoker_docker_start_total",
]

def scrape_prometheus(prometheus_url, session):
    url = f"{prometheus_url.rstrip('/')}/metrics"
    result = {}
    try:
        resp = session.get(url, timeout=10)
        resp.raise_for_status()
        for line in resp.text.splitlines():
            if line.startswith("#"):
                continue
            for name in PROMETHEUS_METRICS_OF_INTEREST:
                if line.startswith(name):
                    parts = line.split()
                    if len(parts) >= 2:
                        try:
                            result[name] = float(parts[-1])
                        except ValueError:
                            pass
    except requests.RequestException as e:
        print(f"  WARN: could not scrape Prometheus at {url}: {e}")
    return result

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ndjson", required=True)
    ap.add_argument("--ow-host", required=True)
    ap.add_argument("--auth", required=True)
    ap.add_argument("--namespace", default="guest")
    ap.add_argument("--prometheus-url", default=None)
    ap.add_argument("--out", default="benchmark_results.json")
    ap.add_argument("--insecure", action="store_true")
    ap.add_argument("--max-activations", type=int, default=None)
    args = ap.parse_args()

    if args.insecure:
        urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

    user, password = args.auth.split(":", 1)
    session = requests.Session()
    session.auth = (user, password)
    session.verify = not args.insecure

    records = []
    with open(args.ndjson) as f:
        for line in f:
            line = line.strip()
            if line:
                records.append(json.loads(line))

    print(f"Loaded {len(records)} records from {args.ndjson}")

    enriched = []
    cold_starts = []
    server_durations_ms = []
    memory_mb_list = []
    wait_times_ms = []

    # Per-function server-side duration tracking
    by_bench_server = {}

    activation_ids = [
        r["activation_id"] for r in records if r.get("activation_id")
    ]
    # Build a map from activation_id to bench name
    act_to_bench = {
        r["activation_id"]: r["bench"]
        for r in records if r.get("activation_id")
    }

    if args.max_activations:
        activation_ids = activation_ids[:args.max_activations]

    print(f"Fetching {len(activation_ids)} activation records from OpenWhisk API...")
    for i, act_id in enumerate(activation_ids):
        if i % 50 == 0:
            print(f"  {i}/{len(activation_ids)}")
        act = fetch_activation(session, args.ow_host, args.namespace, act_id)
        if act is None:
            continue

        annotations = act.get("annotations", [])
        init_time_ms = extract_annotation(annotations, "initTime") or 0
        wait_time_ms = extract_annotation(annotations, "waitTime") or 0
        duration_ms = act.get("duration", 0)
        limits = extract_annotation(annotations, "limits") or {}
        memory_mb = limits.get("memory", 0)
        is_cold = init_time_ms > 0

        bench_name = act_to_bench.get(act_id, "unknown")

        enriched.append({
            "activation_id": act_id,
            "bench": bench_name,
            "is_cold_start": is_cold,
            "init_time_ms": init_time_ms,
            "wait_time_ms": wait_time_ms,
            "server_duration_ms": duration_ms,
            "memory_limit_mb": memory_mb,
        })

        # Track per-function server-side duration
        by_bench_server.setdefault(bench_name, []).append(duration_ms)

        if is_cold:
            cold_starts.append(init_time_ms)
        server_durations_ms.append(duration_ms)
        memory_mb_list.append(memory_mb)
        wait_times_ms.append(wait_time_ms)

        time.sleep(0.01)

    # Client-side latency (per function, for reference only)
    latencies_us = sorted(r["latency_us"] for r in records)
    latencies_ms = [x / 1000 for x in latencies_us]

    by_bench_client = {}
    for r in records:
        if 200 <= r["status_code"] < 300:
            by_bench_client.setdefault(r["bench"], []).append(r["latency_us"] / 1000)

    per_function_stats = {}
    for bench in set(list(by_bench_client.keys()) + list(by_bench_server.keys())):
        client_lats = sorted(by_bench_client.get(bench, []))
        server_durs = sorted(by_bench_server.get(bench, []))
        per_function_stats[bench] = {
            "count": len(server_durs),
            # Server-side execution duration (real)
            "server_p50_ms": percentile(server_durs, 50),
            "server_p90_ms": percentile(server_durs, 90),
            "server_p99_ms": percentile(server_durs, 99),
            "server_mean_ms": statistics.mean(server_durs) if server_durs else 0,
            # Client-side round-trip (for reference, not execution time)
            "client_p50_ms": percentile(client_lats, 50),
            "client_p90_ms": percentile(client_lats, 90),
            "client_p99_ms": percentile(client_lats, 99),
            "client_mean_ms": statistics.mean(client_lats) if client_lats else 0,
        }

    prometheus_snapshot = {}
    if args.prometheus_url:
        print(f"Scraping Prometheus at {args.prometheus_url} ...")
        prometheus_snapshot = scrape_prometheus(args.prometheus_url, session)

    server_durations_sorted = sorted(server_durations_ms)
    cold_starts_sorted = sorted(cold_starts)

    report = {
        "total_invocations": len(records),
        "successful_invocations": sum(1 for r in records if 200 <= r["status_code"] < 300),
        "client_side_latency_ms": {
            "p50": percentile(latencies_ms, 50),
            "p90": percentile(latencies_ms, 90),
            "p99": percentile(latencies_ms, 99),
            "mean": statistics.mean(latencies_ms) if latencies_ms else 0,
            "min": latencies_ms[0] if latencies_ms else 0,
            "max": latencies_ms[-1] if latencies_ms else 0,
            "note": "HTTP round-trip to OW queue, not execution time"
        },
        "server_side_duration_ms": {
            "p50": percentile(server_durations_sorted, 50),
            "p90": percentile(server_durations_sorted, 90),
            "p99": percentile(server_durations_sorted, 99),
            "mean": statistics.mean(server_durations_sorted) if server_durations_sorted else 0,
        },
        "cold_starts": {
            "count": len(cold_starts),
            "rate": len(cold_starts) / len(activation_ids) if activation_ids else 0,
            "p50_init_time_ms": percentile(cold_starts_sorted, 50),
            "p90_init_time_ms": percentile(cold_starts_sorted, 90),
            "p99_init_time_ms": percentile(cold_starts_sorted, 99),
            "mean_init_time_ms": statistics.mean(cold_starts) if cold_starts else 0,
        },
        "wait_time_ms": {
            "p50": percentile(sorted(wait_times_ms), 50),
            "p90": percentile(sorted(wait_times_ms), 90),
            "p99": percentile(sorted(wait_times_ms), 99),
            "note": "Time in OW queue before dispatch"
        },
        "per_function": per_function_stats,
        "prometheus_system_metrics": prometheus_snapshot,
    }

    out_path = Path(args.out)
    with open(out_path, "w") as f:
        json.dump(report, f, indent=2)

    print(f"\nReport written to {out_path}")
    print(f"  Total invocations : {report['total_invocations']}")
    print(f"  Successful        : {report['successful_invocations']}")
    print(f"  Cold starts       : {report['cold_starts']['count']} ({report['cold_starts']['rate']:.1%})")
    print(f"  Server p50/p99 ms : {report['server_side_duration_ms']['p50']:.1f} / {report['server_side_duration_ms']['p99']:.1f}")
    print(f"\nPer-function server-side duration:")
    for bench, stats in sorted(per_function_stats.items()):
        print(f"  {bench:12s}: p50={stats['server_p50_ms']:.1f}ms  p90={stats['server_p90_ms']:.1f}ms  p99={stats['server_p99_ms']:.1f}ms  mean={stats['server_mean_ms']:.1f}ms")

if __name__ == "__main__":
    main()