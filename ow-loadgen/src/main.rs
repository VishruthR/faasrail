mod openwhisk;

use std::{io, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use argh::FromArgs;
use futures::{stream::SelectAll, StreamExt};
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::{broadcast, mpsc},
};
use tokio_stream::wrappers::SignalStream;
use tracing::{error, info, trace, warn};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

use faasrail_loadgen::{
    sink::SinkClient,
    source::{Equidistant, MinuteRange, Poisson, SourceClient, Uniform},
};

use openwhisk::{OpenWhiskSink, OpenWhiskSource};

// ── Defaults ──────────────────────────────────────────────────────────────────

const DEFAULT_OW_HOST: &str = "https://localhost:31001";
const DEFAULT_NAMESPACE: &str = "guest";
const DEFAULT_MINIO_HOSTPORT: &str = "localhost:59000";
const DEFAULT_MINIO_BUCKET: &str = "snaplace-fbpml";

/// Channel capacity shared between OpenWhiskSource workers and the sink.
const RESPONSE_CHANNEL_CAP: usize = 1 << 15;

// ── CLI ───────────────────────────────────────────────────────────────────────

/// ow-loadgen – OpenWhisk load-generator for FaaSRail
///
/// Reads a FaaSRail spec CSV (output of faasrail-shrinkray) and drives
/// invocations against an OpenWhisk deployment, recording each response
/// (latency, status code, activation ID) to a newline-delimited JSON file.
#[derive(Debug, FromArgs)]
struct Cli {
    /// path to the input spec CSV file (output of faasrail-shrinkray)
    #[argh(option)]
    csv: PathBuf,

    /// path to the output file where invocation results will be written (NDJSON)
    #[argh(option, short = 'o')]
    outfile: String,

    /// openWhisk host, e.g. "https://localhost:31001"
    #[argh(option, default = "String::from(DEFAULT_OW_HOST)")]
    ow_host: String,

    /// openWhisk namespace (default: "guest")
    #[argh(option, default = "String::from(DEFAULT_NAMESPACE)")]
    namespace: String,

    /// openWhisk auth in "user:password" format
    #[argh(option, default = "String::from(\"23bc46b1-71f6-4ed5-8c54-816aa4f8c502:123zO3xZCLrMN6v2BKK1dXYFpXlPkccOFqm12CgA2d5AzaP22jaQEe3a6Nk25S9\")")]
    auth: String,

    /// when set, requests use OpenWhisk's blocking mode (latency includes
    /// full execution time); default is non-blocking (dispatch latency only)
    #[argh(switch)]
    blocking: bool,

    /// accept self-signed TLS certificates (useful for local OpenWhisk installs)
    #[argh(switch)]
    insecure: bool,

    /// inter-arrival time distribution: "poisson" (default), "uniform", or "equidistant"
    #[argh(option, default = "String::from(\"poisson\")")]
    iat: String,

    /// seed for the PRNG (default: system entropy; 0 uses a fixed internal seed)
    #[argh(option)]
    seed: Option<u64>,

    /// first invocation ID (default: 0, useful when combining multiple runs)
    #[argh(option, default = "0")]
    invoc_id: u64,

    /// minute range to execute, e.g. "1:10" or "5..15" (default: all minutes)
    #[argh(option, default = "MinuteRange::default()")]
    minutes: MinuteRange,

    /// HOST:PORT of the MinIO server (used to override payload addresses)
    #[argh(option, default = "String::from(DEFAULT_MINIO_HOSTPORT)")]
    minio_address: String,

    /// minIO bucket name (used to override payload bucket references)
    #[argh(option, default = "String::from(DEFAULT_MINIO_BUCKET)")]
    minio_bucket: String,
}

impl Cli {
    fn split_auth(&self) -> Result<(String, String)> {
        let (user, pass) = self
            .auth
            .split_once(':')
            .ok_or_else(|| anyhow!("--auth must be in \"user:password\" format"))?;
        Ok((user.to_owned(), pass.to_owned()))
    }
}

// ── Signal handling ───────────────────────────────────────────────────────────

fn setup_signal_handler(shutdown: broadcast::Sender<()>) -> Result<()> {
    let mut signals = [
        ("ALRM", signal(SignalKind::alarm())),
        ("HUP", signal(SignalKind::hangup())),
        ("INT", signal(SignalKind::interrupt())),
        ("QUIT", signal(SignalKind::quit())),
        ("TERM", signal(SignalKind::terminate())),
        ("USR1", signal(SignalKind::user_defined1())),
        ("USR2", signal(SignalKind::user_defined2())),
        ("PIPE", signal(SignalKind::pipe())),
    ]
    .into_iter()
    .try_fold(SelectAll::new(), |mut acc, (name, s)| {
        acc.push(SignalStream::new(
            s.with_context(|| format!("failed to register SIG{name} handler"))?,
        ));
        Ok::<_, anyhow::Error>(acc)
    })
    .context("failed to set up signal handlers")?;

    tokio::spawn(async move {
        while signals.next().await.is_some() {
            warn!("Signal received; broadcasting shutdown");
            if let Err(err) = shutdown.send(()) {
                error!("Failed to broadcast shutdown: {err:#}");
                panic!("failed to broadcast shutdown: {err:#}");
            }
        }
    });

    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_thread_ids(true)
        .with_line_number(true)
        .try_init()
        .map_err(|err| anyhow!("failed to initialise tracing: {err:#}"))?;

    let cli = argh::from_env::<Cli>();
    trace!("{cli:?}");

    let (user, password) = cli.split_auth()?;

    // Shared channel between all OpenWhiskSource clones and the sink backend.
    let (response_tx, response_rx) = mpsc::channel(RESPONSE_CHANNEL_CAP);

    // Build the source backend.
    let source_backend = OpenWhiskSource::new(
        &cli.ow_host,
        &cli.namespace,
        user,
        password,
        cli.blocking,
        cli.insecure,
        response_tx,
    )
    .context("failed to build OpenWhisk HTTP client")?;

    // Build the sink backend (receives responses and writes them to the output file).
    let sink_backend = OpenWhiskSink::new(response_rx);
    let sink_client =
        SinkClient::new(&cli.outfile, sink_backend).context("failed to create sink client")?;

    let (shutdown, _) = broadcast::channel(1);
    setup_signal_handler(shutdown.clone())?;

    // Spawn the sink.
    let sink = tokio::spawn({
        let shutdown_rx = shutdown.subscribe();
        async move { sink_client.run(shutdown_rx).await }
    });

    // Build the source client (parses the CSV, spawns one worker per function row).
    let mut source_client = match cli.iat.as_str() {
        "uniform" => SourceClient::new(
            &cli.csv,
            None::<&str>,
            cli.seed,
            Uniform,
            cli.invoc_id,
            cli.minutes,
            source_backend,
            &cli.minio_address,
            &cli.minio_bucket,
        ),
        "equidistant" => SourceClient::new(
            &cli.csv,
            None::<&str>,
            cli.seed,
            Equidistant,
            cli.invoc_id,
            cli.minutes,
            source_backend,
            &cli.minio_address,
            &cli.minio_bucket,
        ),
        _ => SourceClient::new(
            &cli.csv,
            None::<&str>,
            cli.seed,
            Poisson,
            cli.invoc_id,
            cli.minutes,
            source_backend,
            &cli.minio_address,
            &cli.minio_bucket,
        ),
    }
    .context("failed to create source client")?;

    // Spawn the source.
    let source = tokio::spawn({
        let shutdown_rx = shutdown.subscribe();
        async move { source_client.run(shutdown_rx).await }
    });

    match tokio::try_join!(source, sink) {
        Ok((source_res, sink_res)) => {
            match source_res {
                Ok(num_requests) => info!(?num_requests, "Source finished"),
                Err(err) => error!("Source finished with error: {err:#}"),
            }
            match sink_res {
                Ok(num_responses) => info!(?num_responses, "Sink finished"),
                Err(err) => error!("Sink finished with error: {err:#}"),
            }
        }
        Err(join_err) => {
            error!("Task join error: {join_err:#}");
            return Err(join_err).context("task join error");
        }
    }

    Ok(())
}
