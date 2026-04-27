extern crate serde_json;
extern crate serde_derive;

use serde::de::Error as _;
use serde_derive::{Deserialize, Serialize};
use serde_json::{Error, Value};
use std::fs::File;
use std::io::{Read, Write};

use wasi::http::outgoing_handler::handle;
use wasi::http::types::{Fields, IncomingResponse, Method, OutgoingRequest, Scheme};
use wasi::io::streams::StreamError;

const MAX_REDIRECTS: usize = 5;

fn host_lc(authority: &str) -> String {
    let a = authority.to_ascii_lowercase();
    if let Some((h, p)) = a.rsplit_once(':') {
        if !a.starts_with('[') && p.chars().all(|c| c.is_ascii_digit()) {
            return h.to_string();
        }
    }
    a
}

fn is_redirect_status(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// Lorem Picsum: first hop is `picsum.photos` → absolute `Location` on `fastly.picsum.photos`.
fn picsum_redirect_allowed(from_authority: &str, location_url: &str) -> Result<(), Error> {
    let (_, loc_auth, _) = parse_url(location_url)?;
    let from_h = host_lc(from_authority);
    let to_h = host_lc(&loc_auth);
    if from_h == "picsum.photos" && to_h == "fastly.picsum.photos" {
        return Ok(());
    }
    if from_h == "fastly.picsum.photos" && to_h == "fastly.picsum.photos" {
        return Ok(());
    }
    Err(Error::custom(format!(
        "picsum redirect rejected (from host `{from_h}` to `{to_h}`)"
    )))
}

fn location_header(response: &IncomingResponse) -> Option<String> {
    let hdrs = response.headers();
    let vals = hdrs.get("location");
    vals.first().and_then(|bytes| {
        std::str::from_utf8(bytes)
            .ok()
            .map(|s| s.trim().to_owned())
    })
}

fn drain_response_body(response: &IncomingResponse) -> Result<(), Error> {
    let body = response
        .consume()
        .map_err(|_| Error::custom("consume redirect body"))?;
    let stream = body
        .stream()
        .map_err(|_| Error::custom("redirect body stream"))?;
    loop {
        match stream.blocking_read(64 * 1024) {
            Ok(chunk) if chunk.is_empty() => break,
            Ok(_) => {}
            Err(StreamError::Closed) => break,
            Err(e) => return Err(Error::custom(format!("redirect body read: {e:?}"))),
        }
    }
    Ok(())
}

fn send_get(url: &str) -> Result<IncomingResponse, Error> {
    let (scheme, authority, path) = parse_url(url)?;

    let headers = Fields::new();
    let request = OutgoingRequest::new(headers);
    request
        .set_method(&Method::Get)
        .map_err(|_| Error::custom("set method"))?;
    request
        .set_scheme(Some(&scheme))
        .map_err(|_| Error::custom("set scheme"))?;
    request
        .set_authority(Some(&authority))
        .map_err(|_| Error::custom("set authority"))?;
    request
        .set_path_with_query(Some(&path))
        .map_err(|_| Error::custom("set path"))?;

    let future = handle(request, None)
        .map_err(|e| Error::custom(format!("outgoing handler: {e:?}")))?;

    future.subscribe().block();

    future
        .get()
        .ok_or_else(|| Error::custom("response future not ready after block"))?
        .map_err(|_| Error::custom("response future already consumed"))?
        .map_err(|e| Error::custom(format!("http error: {e:?}")))
}

const RENDER_WIDTH: u32 = 300;
const RENDER_HEIGHT: u32 = 300;

#[derive(Deserialize)]
struct Input {
    url: String,
    hash: String,
    filename: String,
    max_iter: u32,
    data_dependency_path: Option<String>,
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

/// Parse a URL like `https://example.com/a/b?q=1` into (scheme, authority, path+query).
fn parse_url(url: &str) -> Result<(Scheme, String, String), Error> {
    let (scheme_str, rest) = url
        .split_once("://")
        .ok_or_else(|| Error::custom("url missing scheme"))?;
    let scheme = match scheme_str.to_ascii_lowercase().as_str() {
        "http" => Scheme::Http,
        "https" => Scheme::Https,
        other => return Err(Error::custom(format!("unsupported scheme `{other}`"))),
    };
    let (authority, path_and_query) = match rest.find('/') {
        Some(idx) => (rest[..idx].to_string(), rest[idx..].to_string()),
        None => (rest.to_string(), "/".to_string()),
    };
    if authority.is_empty() {
        return Err(Error::custom("url missing host"));
    }
    Ok((scheme, authority, path_and_query))
}

/// Download `url` to `filepath` using `wasi:http/outgoing-handler`.
/// For **Lorem Picsum** (`picsum.photos`), follows up to `MAX_REDIRECTS` redirects
/// to `fastly.picsum.photos` using absolute `Location` URLs. Other hosts are unchanged
/// (no redirect following).
///
/// Returns the final response status and number of bytes written.
fn get_image(url: &str, filepath: &str) -> Result<(u16, u64), Error> {
    let (_, first_authority, _) = parse_url(url)?;
    let follow_picsum = host_lc(&first_authority) == "picsum.photos";

    let mut current_url = url.to_string();
    for _ in 0..MAX_REDIRECTS {
        let (_, authority, _) = parse_url(&current_url)?;
        let response = send_get(&current_url)?;
        let status = response.status();

        if is_redirect_status(status) && follow_picsum {
            let next = location_header(&response).ok_or_else(|| {
                Error::custom(format!(
                    "HTTP {status} from picsum but no usable Location header"
                ))
            })?;
            picsum_redirect_allowed(&authority, &next)?;
            drain_response_body(&response)?;
            current_url = next;
            continue;
        }

        if is_redirect_status(status) && !follow_picsum {
            drain_response_body(&response)?;
            return Ok((status, 0));
        }

        let body = response
            .consume()
            .map_err(|_| Error::custom("consume response body"))?;
        let stream = body
            .stream()
            .map_err(|_| Error::custom("response body stream"))?;

        let mut file = File::create(filepath)
            .map_err(|e| Error::custom(format!("file create: {e}")))?;

        let mut written: u64 = 0;
        loop {
            match stream.blocking_read(64 * 1024) {
                Ok(chunk) if chunk.is_empty() => break,
                Ok(chunk) => {
                    file.write_all(&chunk)
                        .map_err(|e| Error::custom(format!("file write: {e}")))?;
                    written += chunk.len() as u64;
                }
                Err(StreamError::Closed) => break,
                Err(e) => return Err(Error::custom(format!("body read: {e:?}"))),
            }
        }

        return Ok((status, written));
    }

    Err(Error::custom(format!(
        "more than {MAX_REDIRECTS} redirects (picsum)"
    )))
}

/// Render a Mandelbrot set over the classic view (x in [-2.0, 1.0],
/// y in [-1.2, 1.2]) starting from the top-left pixel. Image bytes are folded
/// into the per-pixel iteration so the work genuinely depends on the file
/// content; the accumulated checksum is black-boxed to prevent DCE.
fn process_image(image: &[u8], width: u32, height: u32, max_iter: u32) {
    let w = width as f64;
    let h = height as f64;
    let n = image.len();
    let mut checksum: u64 = 0;

    for py in 0..height {
        let cy = -1.2 + (py as f64 / h) * 2.4;
        for px in 0..width {
            let cx = -2.0 + (px as f64 / w) * 3.0;
            let byte = if n == 0 {
                0.0
            } else {
                image[((py as usize) * (width as usize) + px as usize) % n] as f64 / 255.0
            };
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

fn does_image_exist(filepath: &str) -> bool {
    File::open(filepath).is_ok()
}

pub fn main(args: Value) -> Result<Value, Error> {
    let input: Input = serde_json::from_value(args)?;
    let start = std::time::Instant::now();

    let filepath = if(input.data_dependency_path.is_some()) {
        input.data_dependency_path.clone().unwrap() + "/" + &input.filename
    } else {
        "./".to_owned() + &input.filename
    };

    let dl_start = std::time::Instant::now();
    // Only skip downloading image if data dependency is defined
    let (status, bytes_written) = if input.data_dependency_path.is_some() && does_image_exist(&filepath) {
        (200u16, 0u64)
    } else {
        get_image(&input.url, &filepath)?
    };
    let download_ms = elapsed_ms(dl_start);

    let mut image = Vec::new();
    File::open(&filepath)
        .and_then(|mut f| f.read_to_end(&mut image))
        .map_err(|e| Error::custom(format!("file read: {e}")))?;

    if image.is_empty() {
        return Err(Error::custom(format!(
            "image is empty (HTTP status {status}, {bytes_written} bytes saved). \
             For non-picsum hosts, redirects are not followed (302 often has no body); \
             use a final URL or pre-seed the file under data_dependency_path."
        )));
    }

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
