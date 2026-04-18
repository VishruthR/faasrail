extern crate bench_disk_rand;
extern crate serde_json;

use serde_json::Value;
use std::io::Read;

const DEFAULT: &str = r#"{"byte_size":4096,"file_size":1}"#;

fn main() {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).expect("read stdin");
    let args: Value = if buf.trim().is_empty() {
        serde_json::from_str(DEFAULT).expect("default json")
    } else {
        serde_json::from_str(buf.trim()).expect("invalid json on stdin")
    };
    let out = bench_disk_rand::main(args).expect("bench failed");
    println!("{}", serde_json::to_string(&out).expect("serialize result"));
}
