extern crate serde_json;
#[macro_use] extern crate serde_derive;

use serde_json::{Error, Value};

#[derive(Deserialize)]
struct Input {
    n: u64,
}

#[derive(Serialize)]
struct Output {
    result: f64,
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

fn float_ops(n: u64) -> f64 {
    let mut result = 0.0_f64;
    for i in 0..n {
        let x = i as f64;
        result += (x.sin() * x.cos()).abs().sqrt();
    }
    result
}

pub fn main(args: Value) -> Result<Value, Error> {
    let input: Input = serde_json::from_value(args)?;
    let start = std::time::Instant::now();
    let result = black_box(float_ops(input.n));
    serde_json::to_value(Output { result, elapsed_ms: elapsed_ms(start) })
}
