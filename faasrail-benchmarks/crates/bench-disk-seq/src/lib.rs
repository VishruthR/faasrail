extern crate serde_json;
extern crate serde_derive;

use serde_derive::{Deserialize, Serialize};
use serde_json::{Error, Value};
use std::fs::{self, File};
use std::io::{Read, Write};

#[derive(Deserialize)]
struct Input {
    byte_size: usize,
    file_size: usize,
}

#[derive(Serialize)]
struct Output {
    write_elapsed_ms: f64,
    read_elapsed_ms: f64,
    total_elapsed_ms: f64,
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

fn fill_bytes(buf: &mut [u8]) {
    for (i, b) in buf.iter_mut().enumerate() {
        let i = i as u32;
        *b = ((i.wrapping_mul(131) ^ 0xA5) % 256) as u8;
    }
}

pub fn main(args: Value) -> Result<Value, Error> {
    let input: Input = serde_json::from_value(args)?;
    let total_bytes = input.file_size * 1024 * 1024;
    let block_size = input.byte_size;
    let path = "./bench_disk_seq.bin";

    let mut block = vec![0u8; block_size];
    fill_bytes(&mut block);

    let write_start = std::time::Instant::now();
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
    let write_elapsed_ms = elapsed_ms(write_start);

    let read_start = std::time::Instant::now();
    {
        let mut f = File::open(path).expect("failed to open file");
        let mut buf = vec![0u8; block_size];
        let mut total_read = 0;
        loop {
            let n = f.read(&mut buf).expect("read failed");
            if n == 0 { break; }
            total_read += n;
        }
        black_box(total_read);
    }
    let read_elapsed_ms = elapsed_ms(read_start);

    let _ = fs::remove_file(path);

    serde_json::to_value(Output {
        write_elapsed_ms,
        read_elapsed_ms,
        total_elapsed_ms: write_elapsed_ms + read_elapsed_ms,
    })
}
