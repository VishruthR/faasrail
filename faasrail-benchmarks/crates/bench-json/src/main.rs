use bench_common::{write_output, Timer};
use serde::Serialize;
use std::hint::black_box;
use std::env;

#[derive(Serialize)]
struct Output {
    parsed: serde_json::Value,
    serialized_length: usize,
    elapsed_ms: f64,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let json_string = &args[1];
    let timer = Timer::start();

    // Deserialize the JSON string into a generic Value
    let parsed: serde_json::Value =
        serde_json::from_str(&json_string).expect("invalid json_string");

    // Re-serialize back to string
    let serialized = serde_json::to_string(&parsed).expect("failed to serialize");
    let len = black_box(serialized.len());

    let elapsed_ms = timer.elapsed_ms();
    write_output(&Output {
        parsed,
        serialized_length: len,
        elapsed_ms,
    });
}
