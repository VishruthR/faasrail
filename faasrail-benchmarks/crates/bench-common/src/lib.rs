use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::{self, Read, Write};

/// Wall-clock timer in milliseconds (floating point).
pub struct Timer(std::time::Instant);

impl Timer {
    pub fn start() -> Self {
        Timer(std::time::Instant::now())
    }

    pub fn elapsed_ms(&self) -> f64 {
        let d = self.0.elapsed();
        d.as_secs_f64() * 1_000.0 + (d.subsec_nanos() as f64) / 1_000_000.0
    }
}

/// Read one JSON object from stdin (trimmed). Used for OpenWhisk param payloads and local runs.
pub fn read_input<T: DeserializeOwned>() -> T {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .expect("read stdin");
    serde_json::from_str(buf.trim()).expect("invalid json input")
}

/// Write JSON result to stdout (single line).
pub fn write_output<T: Serialize>(v: &T) {
    let s = serde_json::to_string(v).expect("serialize output");
    let mut out = io::stdout();
    writeln!(out, "{}", s).expect("write stdout");
    out.flush().expect("flush stdout");
}
