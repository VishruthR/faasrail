extern crate bench_disk_seq;
extern crate serde_json;

fn main() {
    let mut a = std::env::args().skip(1);
    let byte_size: usize = a
        .next()
        .expect("usage: bench-disk-seq <byte_size> <file_size_mib>")
        .parse()
        .expect("<byte_size> must be usize");
    let file_size: usize = a
        .next()
        .expect("usage: bench-disk-seq <byte_size> <file_size_mib>")
        .parse()
        .expect("<file_size_mib> must be usize");
    let args = serde_json::json!({
        "byte_size": byte_size,
        "file_size": file_size,
    });
    let out = bench_disk_seq::main(args).expect("bench_disk_seq::main");
    println!("{}", serde_json::to_string(&out).unwrap());
}
