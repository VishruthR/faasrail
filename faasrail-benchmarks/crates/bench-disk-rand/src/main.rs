use bench_common::{black_box, write_output, Timer};
use rand::Rng;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::env;

#[derive(Serialize)]
struct Output {
    write_elapsed_ms: f64,
    read_elapsed_ms: f64,
    total_elapsed_ms: f64,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let byte_size = args[1].parse::<usize>().unwrap();
    let file_size = args[2].parse::<usize>().unwrap();
    let total_bytes = file_size * 1024 * 1024;
    let block_size = byte_size;
    let num_blocks = total_bytes / block_size;
    let path = "./bench_disk_rand.bin";

    let mut rng = rand::thread_rng();

    // Generate random data block
    let mut block = vec![0u8; block_size];
    getrandom::getrandom(&mut block).expect("getrandom failed");

    // First, create the file with sequential writes so it has the right size
    {
        let mut f = File::create(path).expect("failed to create file");
        for _ in 0..num_blocks {
            f.write_all(&block).expect("write failed");
        }
        f.flush().expect("flush failed");
    }

    // Random write: seek to random block-aligned offsets and write
    let write_timer = Timer::start();
    {
        let mut f = OpenOptions::new()
            .write(true)
            .open(path)
            .expect("failed to open for writing");
        for _ in 0..num_blocks {
            let block_idx = rng.gen_range(0, num_blocks);
            let offset = (block_idx * block_size) as u64;
            f.seek(SeekFrom::Start(offset)).expect("seek failed");
            f.write_all(&block).expect("write failed");
        }
        f.flush().expect("flush failed");
    }
    let write_elapsed_ms = write_timer.elapsed_ms();

    // Random read: seek to random block-aligned offsets and read
    let read_timer = Timer::start();
    {
        let mut f = File::open(path).expect("failed to open for reading");
        let mut buf = vec![0u8; block_size];
        let mut total_read = 0usize;
        for _ in 0..num_blocks {
            let block_idx = rng.gen_range(0, num_blocks);
            let offset = (block_idx * block_size) as u64;
            f.seek(SeekFrom::Start(offset)).expect("seek failed");
            let n = f.read(&mut buf).expect("read failed");
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
