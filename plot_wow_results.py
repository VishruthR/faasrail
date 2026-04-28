import json
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
import numpy as np

with open('/home/svbagga2/faasrail/results/wow_results.json') as f:
    d = json.load(f)

fig, axes = plt.subplots(2, 2, figsize=(14, 10))
fig.suptitle('WOW Benchmark Results (30-min Azure Trace, Single-Node)', fontsize=14, fontweight='bold')

# 1. Per-function invocation counts
ax = axes[0, 0]
funcs = list(d['per_function'].keys())
counts = [d['per_function'][f]['count'] for f in funcs]
colors = ['#4C72B0','#DD8452','#55A868','#C44E52','#8172B2','#937860','#DA8BC3']
bars = ax.bar(funcs, counts, color=colors)
ax.set_title('Invocations per Function')
ax.set_ylabel('Count')
ax.tick_params(axis='x', rotation=30)
for bar, count in zip(bars, counts):
    ax.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 10,
            str(count), ha='center', va='bottom', fontsize=8)

# 2. Per-function client latency percentiles
ax = axes[0, 1]
x = np.arange(len(funcs))
width = 0.25
p50 = [d['per_function'][f]['p50_ms'] for f in funcs]
p90 = [d['per_function'][f]['p90_ms'] for f in funcs]
p99 = [d['per_function'][f]['p99_ms'] for f in funcs]
ax.bar(x - width, p50, width, label='p50', color='#55A868')
ax.bar(x, p90, width, label='p90', color='#DD8452')
ax.bar(x + width, p99, width, label='p99', color='#C44E52')
ax.set_title('Client-Side Latency by Function (ms)')
ax.set_ylabel('Latency (ms)')
ax.set_xticks(x)
ax.set_xticklabels(funcs, rotation=30)
ax.legend()

# 3. Cold start stats
ax = axes[1, 0]
cs = d['cold_starts']
categories = ['p50', 'p90', 'p99', 'mean']
values = [cs['p50_init_time_ms'], cs['p90_init_time_ms'],
          cs['p99_init_time_ms'], cs['mean_init_time_ms']]
bar_colors = ['#55A868', '#DD8452', '#C44E52', '#4C72B0']
bars = ax.bar(categories, values, color=bar_colors)
ax.set_title(f'Cold Start latency (initTime) — {cs["rate"]:.1%} of invocations')
ax.set_ylabel('Init Time (ms)')
for bar, val in zip(bars, values):
    ax.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 1,
            f'{val:.1f}ms', ha='center', va='bottom', fontsize=9)

# 4. Overall stats summary
ax = axes[1, 1]
ax.axis('off')
summary = [
    ['Metric', 'Value'],
    ['Total Invocations', f"{d['total_invocations']:,}"],
    ['Successful', f"{d['successful_invocations']:,} ({d['successful_invocations']/d['total_invocations']:.1%})"],
    ['Failed (429s)', f"{d['total_invocations']-d['successful_invocations']:,} ({(d['total_invocations']-d['successful_invocations'])/d['total_invocations']:.1%})"],
    ['Cold Start Rate', f"{cs['rate']:.1%}"],
    ['Cold Start p50', f"{cs['p50_init_time_ms']:.0f}ms"],
    ['Cold Start mean', f"{cs['mean_init_time_ms']:.0f}ms"],
    ['Server p50', f"{d['server_side_duration_ms']['p50']:.0f}ms"],
    ['Server p90', f"{d['server_side_duration_ms']['p90']:.0f}ms"],
    ['Server p99', f"{d['server_side_duration_ms']['p99']:.0f}ms"],
    ['Wait p50', f"{d['wait_time_ms']['p50']:.0f}ms"],
    ['Wait p99', f"{d['wait_time_ms']['p99']/1000:.1f}s"],
]
table = ax.table(cellText=summary[1:], colLabels=summary[0],
                 loc='center', cellLoc='left')
table.auto_set_font_size(False)
table.set_fontsize(10)
table.scale(1.2, 1.6)
ax.set_title('Summary Statistics')

plt.tight_layout()
plt.savefig('/home/svbagga2/faasrail/results/wow_results.png', dpi=150, bbox_inches='tight')
print("Saved to ~/faasrail/results/wow_results.png")
