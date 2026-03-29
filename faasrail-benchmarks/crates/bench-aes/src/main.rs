use aes::Aes256;
use bench_common::{write_output, Timer};
use ctr::cipher::{KeyIvInit, StreamCipher};
use serde::Serialize;
use std::hint::black_box;
use std::env;

type Aes256Ctr = ctr::Ctr128BE<Aes256>;

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
    let args: Vec<String> = env::args().collect();

    let message_length = args[1].parse::<usize>().unwrap();
    let num_iterations = args[2].parse::<u32>().unwrap();
    let timer = Timer::start();

    // Generate random plaintext
    let mut message = vec![0u8; message_length];
    getrandom::getrandom(&mut message).expect("getrandom failed");

    for _ in 0..num_iterations {
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
    write_output(&Output { success: true, elapsed_ms });
}
