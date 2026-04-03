use bench_common::{black_box, write_output, Timer};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::env;

#[derive(Serialize)]
struct Output {
    original_size: usize,
    compressed_size: usize,
    elapsed_ms: f64,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_size = args[1].parse::<usize>().unwrap();
    let size_bytes = file_size * 1024 * 1024;

    // Generate random data
    let mut data = vec![0u8; size_bytes];
    getrandom::getrandom(&mut data).expect("getrandom failed");

    // Write raw file
    let raw_path = "./bench_gzip_raw.bin";
    let gz_path = "./bench_gzip_compressed.gz";
    fs::write(raw_path, &data).expect("failed to write raw file");

    let timer = Timer::start();

    // Compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&data).expect("gzip write failed");
    let compressed = encoder.finish().expect("gzip finish failed");

    fs::write(gz_path, &compressed).expect("failed to write compressed file");

    let elapsed_ms = timer.elapsed_ms();
    let compressed_size = black_box(compressed.len());

    // Cleanup
    let _ = fs::remove_file(raw_path);
    let _ = fs::remove_file(gz_path);

    write_output(&Output {
        original_size: size_bytes,
        compressed_size,
        elapsed_ms,
    });
}
