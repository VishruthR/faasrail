use bench_common::{black_box, write_output, Timer};
use serde::Serialize;
use std::env;

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
    let args: Vec<String> = env::args().collect();

    let n = args[1].parse::<u64>().unwrap();
    let timer = Timer::start();
    let result = black_box(float_ops(n));
    let elapsed_ms = timer.elapsed_ms();
    write_output(&Output { result, elapsed_ms });
}
