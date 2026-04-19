extern crate bench_json;
extern crate serde_json;

fn main() {
    let json_str = std::env::args()
        .nth(1)
        .expect("usage: bench-json '<json-string>'");
    let args = serde_json::json!({ "json_string": json_str });
    let out = bench_json::main(args).expect("bench_json::main");
    println!("{}", serde_json::to_string(&out).unwrap());
}
