extern crate bench_json;
extern crate serde_json;

use serde_json::Value;
use std::io::Read;

const DEFAULT: &[u8] = br#"{"json_string":"{\"a\":1,\"b\":[1,2,3]}"}"#;

fn main() {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).expect("read stdin");
    let args: Value = if buf.trim().is_empty() {
        serde_json::from_slice(DEFAULT).expect("default json")
    } else {
        serde_json::from_str(buf.trim()).expect("invalid json on stdin")
    };
    let out = bench_json::main(args).expect("bench failed");
    println!("{}", serde_json::to_string(&out).expect("serialize result"));
}
