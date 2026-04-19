extern crate serde_json;
extern crate serde_derive;

use aes::Aes256;
use ctr::cipher::{KeyIvInit, StreamCipher};
use serde_derive::{Deserialize, Serialize};
use serde_json::{Error, Value};

type Aes256Ctr = ctr::Ctr128BE<Aes256>;

#[derive(Deserialize)]
struct Input {
    message_length: usize,
    num_iterations: u32,
}

#[derive(Serialize)]
struct Output {
    success: bool,
    elapsed_ms: f64,
}

/// Fixed 32-byte key (matches the Python original's hardcoded key).
const KEY: &[u8; 32] = b"This is a key123This is a key123";
/// Fixed 16-byte nonce/IV.
const NONCE: &[u8; 16] = b"This is an IV456";

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

    let start = std::time::Instant::now();

    // Generate random plaintext
    let mut message = vec![0u8; input.message_length];
    getrandom::getrandom(&mut message).expect("getrandom failed");

    for _ in 0..input.num_iterations {
        // Encrypt
        let mut ciphertext = message.clone();
        let mut cipher = Aes256Ctr::new(KEY.into(), NONCE.into());
        cipher.apply_keystream(&mut ciphertext);

        // Decrypt
        let mut decrypted = ciphertext;
        let mut cipher = Aes256Ctr::new(KEY.into(), NONCE.into());
        cipher.apply_keystream(&mut decrypted);

        assert_eq!(black_box(&decrypted), &message);
    }

    serde_json::to_value(Output { success: true, elapsed_ms: elapsed_ms(start) })
}
