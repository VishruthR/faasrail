use std::{
    convert::Infallible,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tracing::{error, instrument, warn, Level};

use faasrail_loadgen::{
    sink::SinkBackend,
    source::SourceBackend,
    InvocationId, WorkloadRequest,
};

// ── Response ─────────────────────────────────────────────────────────────────

/// One recorded OpenWhisk invocation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWhiskResponse {
    /// Internal invocation ID assigned by the load generator.
    pub invocation_id: InvocationId,
    /// The benchmark / action name that was called.
    pub bench: String,
    /// Which minute of the experiment this invocation belonged to.
    pub minute: u16,
    /// Unix timestamp (µs) at the moment the HTTP request was dispatched.
    pub issued_at_us: u64,
    /// Round-trip HTTP latency in µs (time until the response was received).
    pub latency_us: u64,
    /// HTTP status code returned by OpenWhisk.
    pub status_code: u16,
    /// OpenWhisk activation ID, present on successful dispatches.
    pub activation_id: Option<String>,
}

// ── Source backend ────────────────────────────────────────────────────────────

/// Source backend that POSTs each invocation to an OpenWhisk deployment and
/// forwards the recorded [`OpenWhiskResponse`] to the companion sink via a
/// shared channel.
///
/// All clones of this struct share the same `reqwest::Client` (connection pool)
/// and the same `mpsc::Sender`, so they all funnel results to one sink.
#[derive(Debug, Clone)]
pub struct OpenWhiskSource {
    client: reqwest::Client,
    host: String,
    namespace: String,
    user: String,
    password: String,
    /// When `true`, the request blocks until the action completes (latency
    /// includes execution time).  When `false` (the default), OpenWhisk
    /// returns an activation ID immediately, giving dispatch latency only.
    blocking: bool,
    response_tx: mpsc::Sender<OpenWhiskResponse>,
}

impl OpenWhiskSource {
    /// Build a new source backend.
    ///
    /// * `danger_accept_invalid_certs` – set to `true` when the OpenWhisk
    ///   endpoint uses a self-signed TLS certificate (common in dev/test).
    pub fn new(
        host: impl Into<String>,
        namespace: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
        blocking: bool,
        danger_accept_invalid_certs: bool,
        response_tx: mpsc::Sender<OpenWhiskResponse>,
    ) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(danger_accept_invalid_certs)
            .build()?;
        Ok(Self {
            client,
            host: host.into(),
            namespace: namespace.into(),
            user: user.into(),
            password: password.into(),
            blocking,
            response_tx,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("HTTP error while invoking OpenWhisk action: {0}")]
    Http(#[from] reqwest::Error),
}

impl SourceBackend for OpenWhiskSource {
    type Error = SourceError;

    #[instrument(level = Level::DEBUG, skip(self, wreq), fields(bench = %wreq.bench))]
    async fn issue(
        &mut self,
        invocation_id: InvocationId,
        wreq: &WorkloadRequest,
        minute: u16,
        timeout: Duration,
    ) -> Result<(), Self::Error> {
        let url = format!(
            "{}/api/v1/namespaces/{}/actions/{}?blocking={}&result=false",
            self.host, self.namespace, wreq.bench, self.blocking,
        );

        // Parse the payload; fall back to an empty object if malformed.
        let payload: serde_json::Value = serde_json::from_str(&wreq.payload)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

        let issued_at_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock went backwards")
            .as_micros() as u64;

        let t_start = Instant::now();
        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.user, Some(&self.password))
            .timeout(timeout)
            .json(&payload)
            .send()
            .await?;
        let latency_us = t_start.elapsed().as_micros() as u64;

        let status_code = resp.status().as_u16();

        // Extract the activation ID from the JSON body (best-effort).
        let activation_id = resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v["activationId"].as_str().map(str::to_owned));

        let ow_resp = OpenWhiskResponse {
            invocation_id,
            bench: wreq.bench.to_string(),
            minute,
            issued_at_us,
            latency_us,
            status_code,
            activation_id,
        };

        if let Err(err) = self.response_tx.try_send(ow_resp) {
            warn!("Response channel full or closed – dropping response: {err:#}");
        }

        Ok(())
    }
}

// ── Sink backend ──────────────────────────────────────────────────────────────

/// Sink backend that receives [`OpenWhiskResponse`]s from [`OpenWhiskSource`]
/// workers (via the shared channel) and forwards them to `SinkClient`'s
/// file-appender.
#[derive(Debug)]
pub struct OpenWhiskSink {
    response_rx: mpsc::Receiver<OpenWhiskResponse>,
}

impl OpenWhiskSink {
    pub fn new(response_rx: mpsc::Receiver<OpenWhiskResponse>) -> Self {
        Self { response_rx }
    }
}

impl SinkBackend for OpenWhiskSink {
    type Error = Infallible;
    type Response = OpenWhiskResponse;

    async fn run(
        mut self,
        to_appender: mpsc::Sender<Self::Response>,
        mut quit_rx: broadcast::Receiver<()>,
    ) -> Result<u64, Self::Error> {
        let mut num_responses = 0u64;

        loop {
            tokio::select! {
                biased;

                // Primary path: forward responses as they arrive.
                msg = self.response_rx.recv() => {
                    match msg {
                        Some(resp) => {
                            num_responses += 1;
                            if let Err(err) = to_appender.send(resp).await {
                                error!("Failed to forward response to file-appender: {err:#}");
                            }
                        }
                        // All OpenWhiskSource clones have been dropped → done.
                        None => break,
                    }
                }

                // Shutdown signal: drain whatever is still buffered, then exit.
                _ = quit_rx.recv() => {
                    while let Ok(resp) = self.response_rx.try_recv() {
                        num_responses += 1;
                        if let Err(err) = to_appender.send(resp).await {
                            error!("Failed to forward response to file-appender during drain: {err:#}");
                        }
                    }
                    break;
                }
            }
        }

        Ok(num_responses)
    }
}
