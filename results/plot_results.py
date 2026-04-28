import json
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
import numpy as np

data = {
  "total_invocations": 14390,
  "successful_invocations": 14390,
  "client_side_latency_ms": {"p50": 2.765, "p90": 5.5832, "p99": 16.829, "mean": 3.665},
  "server_side_duration_ms": {"p50": 6.0, "p90": 245.0, "p99": 269.0, "mean": 42.44},
  "cold_starts": {"count": 18, "rate": 0.00125, "p50_init_time_ms": 29.0, "p90_init_time_ms": 53.7, "mean_init_time_ms": 33.22},
  "wait_time_ms": {"p50": 5.0, "p90": 250.0, "p99": 255.0},
  "per_function": {
    "chameleon": {"count": 1414, "server_p50_ms": 5.0, "server_p90_ms": 6.0, "server_p99_ms": 8.0, "server_mean_ms": 5.11},
    "disk-rand":  {"count": 1974, "server_p50_ms": 76.0, "server_p90_ms": 81.0, "server_p99_ms": 96.3, "server_mean_ms": 77.38},
    "json":       {"count": 684,  "server_p50_ms": 2.0,  "server_p90_ms": 3.0,  "server_p99_ms": 6.0,  "server_mean_ms": 2.39},
    "gzip":       {"count": 1544, "server_p50_ms": 251.0,"server_p90_ms": 268.0,"server_p99_ms": 319.3,"server_mean_ms": 255.53},
    "disk-seq":   {"count": 5420, "server_p50_ms": 4.0,  "server_p90_ms": 5.0,  "server_p99_ms": 7.0,  "server_mean_ms": 3.99},
    "float":      {"count": 1702, "server_p50_ms": 10.0, "server_p90_ms": 11.0, "server_p99_ms": 15.0, "server_mean_ms": 10.04},
    "aes":        {"count": 1652, "server_p50_ms": 9.0,  "server_p90_ms": 11.0, "server_p99_ms": 20.0, "server_mean_ms": 9.62},
  }
}

fns = sorted(data["per_function"].keys(), key=lambda f: data["per_function"][f]["server_mean_ms"])
p50  = [data["per_function"][f]["server_p50_ms"]  for f in fns]
p90  = [data["per_function"][f]["server_p90_ms"]  for f in fns]
p99  = [data["per_function"][f]["server_p99_ms"]  for f in fns]
mean = [data["per_function"][f]["server_mean_ms"] for f in fns]
counts = [data["per_function"][f]["count"] for f in fns]

fig, axes = plt.subplots(2, 2, figsize=(14, 10))
fig.suptitle("WOW Distributed Benchmark", fontsize=14, fontweight="bold")

# 1. Per-function latency (grouped bars)
ax = axes[0, 0]
x = np.arange(len(fns))
w = 0.2
ax.bar(x - 1.5*w, p50,  w, label="p50",  color="#378ADD")
ax.bar(x - 0.5*w, p90,  w, label="p90",  color="#639922")
ax.bar(x + 0.5*w, p99,  w, label="p99",  color="#D85A30")
ax.bar(x + 1.5*w, mean, w, label="mean", color="#7F77DD")
ax.set_xticks(x); ax.set_xticklabels(fns, rotation=30, ha="right")
ax.set_ylabel("ms"); ax.set_title("Server-side latency per function")
ax.legend(fontsize=9); ax.set_yscale("log"); ax.grid(axis="y", alpha=0.3)

# 2. Invocation counts
ax = axes[0, 1]
bars = ax.barh(fns, counts, color="#1D9E75")
ax.set_xlabel("invocations"); ax.set_title("Invocation count per function")
ax.bar_label(bars, fmt="%d", padding=4, fontsize=9)
ax.grid(axis="x", alpha=0.3)

# 3. Overall latency percentiles
ax = axes[1, 0]
cats = ["p50", "p90", "p99"]
srv  = [data["server_side_duration_ms"][k] for k in cats]
wait = [data["wait_time_ms"][k] for k in cats]
cli  = [data["client_side_latency_ms"][k] for k in cats]
x = np.arange(3); w = 0.25
ax.bar(x - w, srv,  w, label="server duration", color="#7F77DD")
ax.bar(x,     wait, w, label="wait (queue)",     color="#888780")
ax.bar(x + w, cli,  w, label="client RTT",       color="#378ADD")
ax.set_xticks(x); ax.set_xticklabels(cats)
ax.set_ylabel("ms"); ax.set_title("Overall latency breakdown")
ax.legend(fontsize=9); ax.grid(axis="y", alpha=0.3)

# 4. Cold start summary
ax = axes[1, 1]
ax.axis("off")
cs = data["cold_starts"]
summary = [
    ("Total invocations",  f"{data['total_invocations']:,}"),
    ("Successful",         f"{data['successful_invocations']:,} (100%)"),
    ("Cold starts",        f"{cs['count']} ({cs['rate']*100:.2f}%)"),
    ("Cold-start p50",     f"{cs['p50_init_time_ms']} ms"),
    ("Cold-start p90",     f"{cs['p90_init_time_ms']} ms"),
    ("Cold-start mean",    f"{cs['mean_init_time_ms']:.1f} ms"),
    ("Server mean",        f"{data['server_side_duration_ms']['mean']:.1f} ms"),
    ("Client RTT mean",    f"{data['client_side_latency_ms']['mean']:.2f} ms"),
]
for i, (label, val) in enumerate(summary):
    ax.text(0.05, 0.92 - i*0.115, label, transform=ax.transAxes,
            fontsize=11, color="gray")
    ax.text(0.65, 0.92 - i*0.115, val, transform=ax.transAxes,
            fontsize=11, fontweight="bold")
ax.set_title("Summary")

plt.tight_layout()
plt.savefig("wow_distributed_results.png", dpi=150, bbox_inches="tight")
plt.show()
print("Saved to wow_distributed_results.png")