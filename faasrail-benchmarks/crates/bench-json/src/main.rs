use bench_common::{read_input, write_output, Timer};
use serde::{Deserialize, Serialize};
use std::hint::black_box;

#[derive(Deserialize)]
struct Input {
    json_string: String,
}

#[derive(Serialize)]
struct Output {
    parsed: serde_json::Value,
    serialized_length: usize,
    elapsed_ms: f64,
}

fn main() {
    let input: Input = read_input();
    let timer = Timer::start();

    // Deserialize the JSON string into a generic Value
    let parsed: serde_json::Value =
        serde_json::from_str(&input.json_string).expect("invalid json_string");

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
