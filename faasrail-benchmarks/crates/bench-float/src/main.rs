use bench_common::{read_input, write_output, Timer};
use serde::{Deserialize, Serialize};
use std::hint::black_box;

#[derive(Deserialize)]
struct Input {
    n: u64,
}

#[derive(Serialize)]
struct Output {
    result: f64,
    elapsed_ms: f64,
}

fn float_ops(n: u64) -> f64 {
    let mut result = 0.0_f64;
    for i in 0..n {
        let x = i as f64;
        result += (x.sin() * x.cos()).abs().sqrt();
    }
    result
}

fn main() {
    let input: Input = read_input();
    let timer = Timer::start();
    let result = black_box(float_ops(input.n));
    let elapsed_ms = timer.elapsed_ms();
    write_output(&Output { result, elapsed_ms });
}
