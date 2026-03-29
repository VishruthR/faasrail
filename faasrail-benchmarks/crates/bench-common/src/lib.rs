use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::{self, Read};
use std::time::Instant;

/// Read JSON input from stdin and deserialize into `T`.
pub fn read_input<T: DeserializeOwned>() -> T {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).expect("failed to read stdin");
    serde_json::from_str(&buf).expect("invalid JSON input")
}

/// Serialize `value` as JSON and write to stdout.
pub fn write_output<T: Serialize>(value: &T) {
    let out = serde_json::to_string(value).expect("failed to serialize output");
    println!("{out}");
}

/// Simple timer wrapping `std::time::Instant`.
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn start() -> Self {
        Self { start: Instant::now() }
    }

    /// Returns elapsed time in milliseconds as f64.
    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}
