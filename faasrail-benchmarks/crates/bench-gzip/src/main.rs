extern crate bench_gzip;
extern crate serde_json;

fn main() {
    let file_size: usize = std::env::args()
        .nth(1)
        .expect("usage: bench-gzip <file_size_mib>")
        .parse()
        .expect("<file_size_mib> must be usize");
    let args = serde_json::json!({ "file_size": file_size });
    let out = bench_gzip::main(args).expect("bench_gzip::main");
    println!("{}", serde_json::to_string(&out).unwrap());
}
