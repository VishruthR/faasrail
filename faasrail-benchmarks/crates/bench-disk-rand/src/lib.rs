extern crate serde_json;
extern crate serde_derive;

use serde_derive::{Deserialize, Serialize};
use serde_json::{Error, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

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

/// Tiny deterministic PRNG (no `rand` crate); good enough for seek offsets.
struct Lcg64(u64);

impl Lcg64 {
    fn new() -> Self {
        Lcg64(0x243F_6A88_85A3_08D3)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0
    }
    fn gen_below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() as usize) % n
    }
}

pub fn main(args: Value) -> Result<Value, Error> {
    let input: Input = serde_json::from_value(args)?;
    let total_bytes = input.file_size * 1024 * 1024;
    let block_size = input.byte_size;
    let num_blocks = total_bytes / block_size;
    let path = "./bench_disk_rand.bin";

    let mut rng = Lcg64::new();
    let mut block = vec![0u8; block_size];
    fill_bytes(&mut block);

    {
        let mut f = File::create(path).expect("failed to create file");
        for _ in 0..num_blocks {
            f.write_all(&block).expect("write failed");
        }
        f.flush().expect("flush failed");
    }

    let write_start = std::time::Instant::now();
    {
        let mut f = OpenOptions::new()
            .write(true)
            .open(path)
            .expect("failed to open for writing");
        for _ in 0..num_blocks {
            let offset = (rng.gen_below(num_blocks) * block_size) as u64;
            f.seek(SeekFrom::Start(offset)).expect("seek failed");
            f.write_all(&block).expect("write failed");
        }
        f.flush().expect("flush failed");
    }
    let write_elapsed_ms = elapsed_ms(write_start);

    let read_start = std::time::Instant::now();
    {
        let mut f = File::open(path).expect("failed to open for reading");
        let mut buf = vec![0u8; block_size];
        let mut total_read = 0usize;
        for _ in 0..num_blocks {
            let offset = (rng.gen_below(num_blocks) * block_size) as u64;
            f.seek(SeekFrom::Start(offset)).expect("seek failed");
            total_read += f.read(&mut buf).expect("read failed");
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
