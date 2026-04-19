extern crate bench_float;
extern crate serde_json;

fn main() {
    let mut a = std::env::args().skip(1);
    let n: u64 = a
        .next()
        .expect("usage: bench-float <n>")
        .parse()
        .expect("<n> must be a u64");
    let args = serde_json::json!({ "n": n });
    let out = bench_float::main(args).expect("bench_float::main");
    println!("{}", serde_json::to_string(&out).unwrap());
}
