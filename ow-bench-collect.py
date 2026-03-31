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
from urllib.parse import urljoin

import requests
import urllib3

# ── Percentile helper ─────────────────────────────────────────────────────────

def percentile(sorted_values: list[float], p: float) -> float:
    if not sorted_values:
        return 0.0
    k = (len(sorted_values) - 1) * p / 100
    lo, hi = int(k), min(int(k) + 1, len(sorted_values) - 1)
    return sorted_values[lo] + (sorted_values[hi] - sorted_values[lo]) * (k - lo)


# ── OpenWhisk Activation API ──────────────────────────────────────────────────

def fetch_activation(session: requests.Session, ow_host: str,
                     namespace: str, activation_id: str) -> dict | None:
    url = f"{ow_host}/api/v1/namespaces/{namespace}/activations/{activation_id}"
    try:
        resp = session.get(url, timeout=10)
        resp.raise_for_status()
        return resp.json()
    except requests.RequestException as e:
        print(f"  WARN: could not fetch activation {activation_id}: {e}")
        return None


def extract_annotation(annotations: list[dict], key: str):
    for ann in annotations:
        if ann.get("key") == key:
            return ann.get("value")
    return None


# ── Prometheus scraper ────────────────────────────────────────────────────────

PROMETHEUS_METRICS_OF_INTEREST = [
    "openwhisk_invoker_containerPool_activeActivations",
    "openwhisk_invoker_containerPool_busyContainersTotal",
    "openwhisk_invoker_containerPool_freeContainersTotal",
    "openwhisk_invoker_containerPool_memoryUsedMBytes",
    "openwhisk_invoker_containerPool_memoryQueuedMBytes",
    "openwhisk_invoker_activationsTotal",
]

def scrape_prometheus(prometheus_url: str, session: requests.Session) -> dict:
    """
    Scrapes the /metrics text endpoint and extracts the metrics we care about.
    Returns a dict of {metric_name: float}.
    """
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


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ndjson", required=True, help="Path to results.ndjson from ow-loadgen")
    ap.add_argument("--ow-host", required=True)
    ap.add_argument("--auth", required=True, help="user:password")
    ap.add_argument("--namespace", default="guest")
    ap.add_argument("--prometheus-url", default=None,
                    help="Base URL of Prometheus /metrics endpoint, e.g. http://invoker:9095")
    ap.add_argument("--out", default="benchmark_results.json")
    ap.add_argument("--insecure", action="store_true")
    ap.add_argument("--max-activations", type=int, default=None,
                    help="Limit activation API calls (for quick testing)")
    args = ap.parse_args()

    if args.insecure:
        urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

    user, password = args.auth.split(":", 1)
    session = requests.Session()
    session.auth = (user, password)
    session.verify = not args.insecure

    # ── Load NDJSON ──────────────────────────────────────────────────────────
    records = []
    with open(args.ndjson) as f:
        for line in f:
            line = line.strip()
            if line:
                records.append(json.loads(line))

    print(f"Loaded {len(records)} records from {args.ndjson}")

    # ── Enrich via Activation API ────────────────────────────────────────────
    enriched = []
    cold_starts = []
    server_durations_ms = []
    memory_mb_list = []
    wait_times_ms = []

    activation_ids = [
        r["activation_id"] for r in records if r.get("activation_id")
    ]
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

        # memory is in the limits annotation
        limits = extract_annotation(annotations, "limits") or {}
        memory_mb = limits.get("memory", 0)

        is_cold = init_time_ms > 0

        enriched.append({
            "activation_id": act_id,
            "is_cold_start": is_cold,
            "init_time_ms": init_time_ms,
            "wait_time_ms": wait_time_ms,
            "server_duration_ms": duration_ms,
            "memory_limit_mb": memory_mb,
        })

        if is_cold:
            cold_starts.append(init_time_ms)
        server_durations_ms.append(duration_ms)
        memory_mb_list.append(memory_mb)
        wait_times_ms.append(wait_time_ms)

        # Be gentle with the OW API
        time.sleep(0.01)

    # ── Compute client-side latency percentiles ───────────────────────────────
    latencies_us = sorted(r["latency_us"] for r in records)
    latencies_ms = [x / 1000 for x in latencies_us]

    # Per-function breakdown
    by_bench: dict[str, list[float]] = {}
    for r in records:
        by_bench.setdefault(r["bench"], []).append(r["latency_us"] / 1000)
    per_function_stats = {}
    for bench, lats in by_bench.items():
        lats_sorted = sorted(lats)
        per_function_stats[bench] = {
            "count": len(lats_sorted),
            "p50_ms": percentile(lats_sorted, 50),
            "p90_ms": percentile(lats_sorted, 90),
            "p99_ms": percentile(lats_sorted, 99),
            "mean_ms": statistics.mean(lats_sorted),
        }

    # ── Optionally scrape Prometheus ─────────────────────────────────────────
    prometheus_snapshot = {}
    if args.prometheus_url:
        print(f"Scraping Prometheus at {args.prometheus_url} ...")
        prometheus_snapshot = scrape_prometheus(args.prometheus_url, session)

    # ── Assemble report ──────────────────────────────────────────────────────
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
        },
        "per_function": per_function_stats,
        "prometheus_system_metrics": prometheus_snapshot,
    }

    out_path = Path(args.out)
    with open(out_path, "w") as f:
        json.dump(report, f, indent=2)

    print(f"\nReport written to {out_path}")
    print(f"  Total invocations : {report['total_invocations']}")
    print(f"  Cold starts       : {report['cold_starts']['count']} ({report['cold_starts']['rate']:.1%})")
    print(f"  Client p50/p99 ms : {report['client_side_latency_ms']['p50']:.1f} / {report['client_side_latency_ms']['p99']:.1f}")
    if server_durations_sorted:
        print(f"  Server p50/p99 ms : {report['server_side_duration_ms']['p50']:.1f} / {report['server_side_duration_ms']['p99']:.1f}")


if __name__ == "__main__":
    main()