use bench_common::{read_input, write_output, Timer};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::hint::black_box;
use std::io::{Read, Write};

#[derive(Deserialize)]
struct Input {
    /// File size in MB.
    file_size: usize,
    /// Block size for each sequential read/write in bytes.
    byte_size: usize,
}

#[derive(Serialize)]
struct Output {
    write_elapsed_ms: f64,
    read_elapsed_ms: f64,
    total_elapsed_ms: f64,
}

fn main() {
    let input: Input = read_input();
    let total_bytes = input.file_size * 1024 * 1024;
    let block_size = input.byte_size;
    let path = "/tmp/bench_disk_seq.bin";

    // Generate one block of random data to write repeatedly
    let mut block = vec![0u8; block_size];
    getrandom::getrandom(&mut block).expect("getrandom failed");

    // Sequential write
    let write_timer = Timer::start();
    {
        let mut f = File::create(path).expect("failed to create file");
        let mut written = 0;
        while written < total_bytes {
            let to_write = block_size.min(total_bytes - written);
            f.write_all(&block[..to_write]).expect("write failed");
            written += to_write;
        }
        f.flush().expect("flush failed");
    }
    let write_elapsed_ms = write_timer.elapsed_ms();

    // Sequential read
    let read_timer = Timer::start();
    {
        let mut f = File::open(path).expect("failed to open file");
        let mut buf = vec![0u8; block_size];
        let mut total_read = 0;
        loop {
            let n = f.read(&mut buf).expect("read failed");
            if n == 0 {
                break;
            }
            total_read += n;
        }
        black_box(total_read);
    }
    let read_elapsed_ms = read_timer.elapsed_ms();

    let _ = fs::remove_file(path);

    write_output(&Output {
        write_elapsed_ms,
        read_elapsed_ms,
        total_elapsed_ms: write_elapsed_ms + read_elapsed_ms,
    });
}
