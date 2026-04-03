extern crate serde_json;
extern crate serde_derive;

use serde_derive::{Deserialize, Serialize};
use serde_json::{Error, Value};

#[derive(Deserialize)]
struct Input {
    json_string: String,
}

#[derive(Serialize)]
struct Output {
    parsed: Value,
    serialized_length: usize,
    elapsed_ms: f64,
}

fn elapsed_ms(start: std::time::Instant) -> f64 {
    let d = start.elapsed();
    (d.as_secs() as f64) * 1_000.0 + (d.subsec_nanos() as f64) / 1_000_000.0
}

#[inline]
fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = std::ptr::read_volatile(&dummy as *const T);
        std::mem::forget(dummy);
        ret
    }
}

pub fn main(args: Value) -> Result<Value, Error> {
    let input: Input = serde_json::from_value(args)?;
    let start = std::time::Instant::now();
    let parsed: Value = serde_json::from_str(&input.json_string)?;
    let serialized = serde_json::to_string(&parsed)?;
    let serialized_length = black_box(serialized.len());
    serde_json::to_value(Output { parsed, serialized_length, elapsed_ms: elapsed_ms(start) })
}
