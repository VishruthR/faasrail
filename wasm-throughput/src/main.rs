use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use base64::Engine;
use clap::Parser;
use rand::seq::SliceRandom;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration, Instant};

const INVOCATION_TIMEOUT: Duration = Duration::from_secs(600);

// Bench prefix → OpenWhisk action path (longest-first match).
const BENCH_TO_ACTION: &[(&str, &str)] = &[
    ("disk-rand", "/guest/bench_disk_rand"),
    ("disk-seq", "/guest/bench_disk_seq"),
    ("chameleon", "/guest/bench_chameleon"),
    ("float", "/guest/bench_float"),
    ("gzip", "/guest/bench_gzip"),
    ("json", "/guest/bench_json"),
    ("aes", "/guest/bench_aes"),
];

#[derive(Parser)]
#[command(
    name = "wasm-throughput",
    about = "OpenWhisk load tool: RPS ramp using first-minute trace mixture"
)]
struct Args {
    /// Path to the trace spec CSV (same format as wasm-loadgen / shrinkray)
    #[arg(short, long)]
    trace: String,

    /// Requests per second during the first wall-clock minute
    #[arg(long, default_value_t = 5)]
    start_rps: u32,

    /// Added to RPS after each full minute
    #[arg(long, default_value_t = 1)]
    rps_increment: u32,

    /// Max rows to keep per benchmark type (same semantics as wasm-loadgen)
    #[arg(long, default_value_t = 5)]
    max_per_bench: usize,

    /// Max concurrent in-flight HTTP requests
    #[arg(long, default_value_t = 50)]
    concurrency: usize,

    /// Accept invalid TLS certificates (common with self-hosted OpenWhisk)
    #[arg(long, default_value_t = true)]
    insecure: bool,

    /// Print mixture + first-minute plan and exit without invoking actions
    #[arg(long)]
    dry_run: bool,
}

struct WskConfig {
    api_host: String,
    auth_header: String,
}

struct TraceRow {
    bench_type: String,
    action_path: String,
    payload: Value,
    rpm: Vec<u32>,
}

fn parse_wskprops() -> WskConfig {
    let home = std::env::var("HOME").expect("HOME not set");
    let path = format!("{home}/.wskprops");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));

    let mut api_host = None;
    let mut auth = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("APIHOST=") {
            api_host = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("AUTH=") {
            auth = Some(val.trim().to_string());
        }
    }

    let mut api_host = api_host.expect("APIHOST not found in ~/.wskprops");
    let auth = auth.expect("AUTH not found in ~/.wskprops");
    let b64 = base64::engine::general_purpose::STANDARD.encode(&auth);

    if !api_host.starts_with("http://") && !api_host.starts_with("https://") {
        api_host = format!("https://{api_host}");
    }

    WskConfig {
        api_host: api_host.trim_end_matches('/').to_string(),
        auth_header: format!("Basic {b64}"),
    }
}

fn resolve_action(bench_name: &str) -> Option<(&'static str, &'static str)> {
    for &(prefix, action) in BENCH_TO_ACTION {
        if bench_name.starts_with(prefix)
            && bench_name.as_bytes().get(prefix.len()) == Some(&b'-')
        {
            return Some((prefix, action));
        }
    }
    None
}

fn parse_trace(path: &str) -> Vec<TraceRow> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .unwrap_or_else(|e| panic!("Failed to open {path}: {e}"));

    let mut rows = Vec::new();

    for (line_no, result) in reader.records().enumerate() {
        let record =
            result.unwrap_or_else(|e| panic!("CSV parse error at line {}: {e}", line_no + 2));

        let wreq_json: Value = serde_json::from_str(&record[1])
            .unwrap_or_else(|e| panic!("Bad mapped_wreq JSON at line {}: {e}", line_no + 2));

        let bench = wreq_json["bench"]
            .as_str()
            .unwrap_or_else(|| panic!("Missing bench field at line {}", line_no + 2));

        let payload_str = wreq_json["payload"]
            .as_str()
            .unwrap_or_else(|| panic!("Missing payload field at line {}", line_no + 2));

        let payload: Value = serde_json::from_str(payload_str)
            .unwrap_or_else(|e| panic!("Bad payload JSON at line {}: {e}", line_no + 2));

        let (bench_type, action_path) = match resolve_action(bench) {
            Some((bt, ap)) => (bt.to_string(), ap.to_string()),
            None => {
                eprintln!("warning: unknown benchmark '{bench}', skipping");
                continue;
            }
        };

        let num_minute_cols = record.len() - 2;
        let rpm: Vec<u32> = (0..num_minute_cols)
            .map(|i| record[i + 2].parse::<u32>().unwrap_or(0))
            .collect();

        rows.push(TraceRow {
            bench_type,
            action_path,
            payload,
            rpm,
        });
    }

    rows
}

fn filter_rows(rows: Vec<TraceRow>, max_per_bench: usize) -> Vec<TraceRow> {
    let mut groups: HashMap<String, Vec<TraceRow>> = HashMap::new();
    for row in rows {
        groups.entry(row.bench_type.clone()).or_default().push(row);
    }

    let mut filtered = Vec::new();
    let mut types: Vec<_> = groups.keys().cloned().collect();
    types.sort();

    for bench_type in &types {
        let mut group = groups.remove(bench_type).unwrap();
        group.sort_by(|a, b| {
            let sum_b: u64 = b.rpm.iter().map(|&v| v as u64).sum();
            let sum_a: u64 = a.rpm.iter().map(|&v| v as u64).sum();
            sum_b.cmp(&sum_a)
        });
        let take = max_per_bench.min(group.len());
        eprintln!("  {bench_type}: {take}/{} rows", group.len());
        filtered.extend(group.into_iter().take(take));
    }

    filtered
}

fn invocation_url(api_host: &str, action_path: &str) -> String {
    let trimmed = action_path.trim_start_matches('/');
    let (namespace, action) = trimmed
        .split_once('/')
        .unwrap_or_else(|| panic!("Bad action path: {action_path}"));
    format!("{api_host}/api/v1/namespaces/{namespace}/actions/{action}")
}

/// Split `n` invocations across rows using largest-remainder from first-minute weights.
fn allocate_counts(n: usize, weights: &[u32]) -> Vec<usize> {
    let sum_w: u128 = weights.iter().map(|&w| w as u128).sum();
    assert!(sum_w > 0, "allocate_counts: zero total weight");

    #[derive(Clone, Copy)]
    struct Part {
        idx: usize,
        floor: usize,
        rem: u128,
    }

    let mut parts: Vec<Part> = weights
        .iter()
        .enumerate()
        .map(|(idx, &w)| {
            let prod = n as u128 * w as u128;
            let floor = (prod / sum_w) as usize;
            let rem = prod % sum_w;
            Part { idx, floor, rem }
        })
        .collect();

    let mut counts = vec![0usize; weights.len()];
    let mut assigned = 0usize;
    for p in &parts {
        counts[p.idx] = p.floor;
        assigned += p.floor;
    }

    let mut leftover = n.saturating_sub(assigned);
    parts.sort_by_key(|p| std::cmp::Reverse(p.rem));
    let mut i = 0;
    while leftover > 0 {
        counts[parts[i % parts.len()].idx] += 1;
        leftover -= 1;
        i += 1;
    }

    counts
}

fn print_mixture(rows: &[TraceRow], weights: &[u32]) {
    let sum: u64 = weights.iter().map(|&w| w as u64).sum();
    eprintln!("First-minute mixture (sum={sum} trace invocations):");
    let mut by_bench: HashMap<&str, u64> = HashMap::new();
    for (row, &w) in rows.iter().zip(weights.iter()) {
        *by_bench.entry(row.bench_type.as_str()).or_default() += w as u64;
    }
    let mut pairs: Vec<_> = by_bench.into_iter().collect();
    pairs.sort_by_key(|(k, _)| *k);
    for (name, w) in pairs {
        let pct = 100.0 * w as f64 / sum as f64;
        eprintln!("  {name}: {w} ({pct:.2}%)");
    }
}

struct Invocation {
    action_path: String,
    payload: Value,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config = parse_wskprops();
    eprintln!("OpenWhisk API: {}", config.api_host);

    eprintln!("Loading trace: {}", args.trace);
    let rows = parse_trace(&args.trace);
    let trace_minutes = rows.first().map(|r| r.rpm.len()).unwrap_or(0);
    eprintln!("Parsed {} rows, {} minute columns\n", rows.len(), trace_minutes);

    if trace_minutes == 0 {
        panic!("Trace has no minute columns");
    }

    eprintln!("Filtering (max {} per bench type):", args.max_per_bench);
    let rows = filter_rows(rows, args.max_per_bench);
    eprintln!("  total: {} rows\n", rows.len());

    let weights: Vec<u32> = rows.iter().map(|r| *r.rpm.first().unwrap_or(&0)).collect();
    let sum_w: u64 = weights.iter().map(|&w| w as u64).sum();
    if sum_w == 0 {
        panic!(
            "No invocations in the first trace minute (first RPM column). \
             Choose a trace whose first minute column has nonzero traffic."
        );
    }

    print_mixture(&rows, &weights);

    if args.dry_run {
        let n = args.start_rps as usize * 60;
        let counts = allocate_counts(n, &weights);
        eprintln!("\nDry run — example minute 0: target {} invocations", n);
        for (row, &c) in rows.iter().zip(counts.iter()) {
            if c > 0 {
                eprintln!("  {} {} invocations", row.bench_type, c);
            }
        }
        eprintln!("Exiting.");
        return;
    }

    let client = Client::builder()
        .danger_accept_invalid_certs(args.insecure)
        .pool_max_idle_per_host(args.concurrency)
        .build()
        .expect("Failed to create HTTP client");

    let semaphore = Arc::new(Semaphore::new(args.concurrency));
    let total_ok = Arc::new(AtomicU64::new(0));
    let total_err = Arc::new(AtomicU64::new(0));

    let experiment_start = Instant::now();
    let mut wall_minute: u64 = 0;

    loop {
        let rps = args.start_rps.saturating_add(wall_minute as u32 * args.rps_increment);
        let n = rps as usize * 60;
        let minute_start = Instant::now();

        let counts = allocate_counts(n, &weights);
        let mut invocations: Vec<Invocation> = Vec::with_capacity(n);
        for (row, &c) in rows.iter().zip(counts.iter()) {
            for _ in 0..c {
                invocations.push(Invocation {
                    action_path: row.action_path.clone(),
                    payload: row.payload.clone(),
                });
            }
        }

        debug_assert_eq!(invocations.len(), n);

        invocations.shuffle(&mut rand::thread_rng());

        eprintln!(
            "[wall minute {:>4}] target {} invocations ({rps} RPS)",
            wall_minute + 1,
            n,
        );

        for (i, inv) in invocations.into_iter().enumerate() {
            let target = minute_start + Duration::from_secs_f64(60.0 * i as f64 / n as f64);
            let now = Instant::now();
            if target > now {
                sleep(target - now).await;
            }

            let client = client.clone();
            let auth = config.auth_header.clone();
            let host = config.api_host.clone();
            let sem = semaphore.clone();
            let ok = total_ok.clone();
            let err = total_err.clone();

            tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let url = invocation_url(&host, &inv.action_path);

                let result = client
                    .post(&url)
                    .header("Authorization", &auth)
                    .timeout(INVOCATION_TIMEOUT)
                    .json(&inv.payload)
                    .send()
                    .await;

                match result {
                    Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 202 => {
                        ok.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        eprintln!("  WARN {}: {} — {}", inv.action_path, status, body);
                        err.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("  ERROR {}: {e}", inv.action_path);
                        err.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }

        let elapsed = minute_start.elapsed();
        if elapsed < Duration::from_secs(60) {
            sleep(Duration::from_secs(60) - elapsed).await;
        }

        eprintln!(
            "  cumulative ok={}, err={}",
            total_ok.load(Ordering::Relaxed),
            total_err.load(Ordering::Relaxed),
        );

        wall_minute += 1;

        if wall_minute.is_multiple_of(10) {
            let elapsed = experiment_start.elapsed();
            eprintln!(
                "  (running {:.0}s — Ctrl+C to stop)",
                elapsed.as_secs_f64()
            );
        }
    }
}
