extern crate serde_json;
extern crate getrandom;
extern crate flate2;
#[macro_use] extern crate serde_derive;

use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::{Error, Value};
use std::io::Write;

#[derive(Deserialize)]
struct Input {
    file_size: usize,
}

#[derive(Serialize)]
struct Output {
    original_size: usize,
    compressed_size: usize,
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
    let size_bytes = input.file_size * 1024 * 1024;

    let mut data = vec![0u8; size_bytes];
    getrandom::getrandom(&mut data).expect("getrandom failed");

    let start = std::time::Instant::now();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&data).expect("gzip write failed");
    let compressed = encoder.finish().expect("gzip finish failed");
    let compressed_size = black_box(compressed.len());

    serde_json::to_value(Output {
        original_size: size_bytes,
        compressed_size,
        elapsed_ms: elapsed_ms(start),
    })
}
