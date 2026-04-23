extern crate serde_json;
extern crate serde_derive;

use serde::de::Error as _;
use serde_derive::{Deserialize, Serialize};
use serde_json::{Error, Value};
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;

const RENDER_WIDTH: u32 = 300;
const RENDER_HEIGHT: u32 = 300;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Deserialize)]
struct Input {
    url: String,
    hash: String,
    filename: String,
    max_iter: u32,
}

#[derive(Serialize)]
struct Output {
    bytes_written: u64,
    download_ms: f64,
    render_ms: f64,
    elapsed_ms: f64,
}

fn elapsed_ms(start: std::time::Instant) -> f64 {
    let d = start.elapsed();
    (d.as_secs() as f64) * 1_000.0 + (d.subsec_nanos() as f64) / 1_000_000.0
}

/// Download `url` to `filename` using `reqwest`'s blocking client.
/// Returns the response status and number of bytes written.
fn get_image(url: &str, filename: &str) -> Result<(u16, u64), Error> {
    let client = reqwest::blocking::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .map_err(|e| Error::custom(format!("build http client: {e}")))?;

    let mut response = client
        .get(url)
        .send()
        .map_err(|e| Error::custom(format!("http get: {e}")))?;
    let status = response.status().as_u16();

    let mut file = File::create(filename)
        .map_err(|e| Error::custom(format!("file create: {e}")))?;

    let mut buf = [0u8; 64 * 1024];
    let mut written: u64 = 0;
    loop {
        let n = response
            .read(&mut buf)
            .map_err(|e| Error::custom(format!("body read: {e}")))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| Error::custom(format!("file write: {e}")))?;
        written += n as u64;
    }

    Ok((status, written))
}

/// Render a Mandelbrot set over the classic view (x in [-2.0, 1.0],
/// y in [-1.2, 1.2]) starting from the top-left pixel. Image bytes are folded
/// into the per-pixel iteration so the work genuinely depends on the file
/// content; the accumulated checksum is black-boxed to prevent DCE.
fn process_image(image: &[u8], width: u32, height: u32, max_iter: u32) {
    let w = width as f64;
    let h = height as f64;
    let n = image.len().max(1);
    let mut checksum: u64 = 0;

    for py in 0..height {
        let cy = -1.2 + (py as f64 / h) * 2.4;
        for px in 0..width {
            let cx = -2.0 + (px as f64 / w) * 3.0;
            let byte = image[((py as usize) * (width as usize) + px as usize) % n] as f64 / 255.0;
            let cx = cx + byte * 1e-6;

            let (mut x, mut y) = (0.0_f64, 0.0_f64);
            let mut i = 0u32;
            while i < max_iter && x * x + y * y <= 4.0 {
                let xt = x * x - y * y + cx;
                y = 2.0 * x * y + cy;
                x = xt;
                i += 1;
            }
            checksum = checksum.wrapping_add(i as u64);
        }
    }

    std::hint::black_box(checksum);
}

fn does_image_exist(filename: &str) -> bool {
    File::open(filename).is_ok()
}

pub fn main(args: Value) -> Result<Value, Error> {
    let input: Input = serde_json::from_value(args)?;
    let _ = input.hash; // reserved for future integrity check; keep field required
    let start = std::time::Instant::now();

    let dl_start = std::time::Instant::now();
    let (_status, bytes_written) = if does_image_exist(&input.filename) {
        (200, 0)
    } else {
        get_image(&input.url, &input.filename)?
    };
    let download_ms = elapsed_ms(dl_start);

    let mut image = Vec::new();
    File::open(&input.filename)
        .and_then(|mut f| f.read_to_end(&mut image))
        .map_err(|e| Error::custom(format!("file read: {e}")))?;

    let render_start = std::time::Instant::now();
    process_image(&image, RENDER_WIDTH, RENDER_HEIGHT, input.max_iter);
    let render_ms = elapsed_ms(render_start);

    serde_json::to_value(Output {
        bytes_written,
        download_ms,
        render_ms,
        elapsed_ms: elapsed_ms(start),
    })
}
