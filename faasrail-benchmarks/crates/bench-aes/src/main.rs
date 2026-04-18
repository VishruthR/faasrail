use aes::Aes256;
use bench_common::{read_input, write_output, Timer};
use ctr::cipher::{KeyIvInit, StreamCipher};
use serde_derive::{Deserialize, Serialize};
use std::hint::black_box;

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

fn main() {
    let input: Input = read_input();
    let timer = Timer::start();

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

    let elapsed_ms = timer.elapsed_ms();
    write_output(&Output {
        success: true,
        elapsed_ms,
    });
}
