extern crate bench_aes;
extern crate serde_json;

fn main() {
    let mut a = std::env::args().skip(1);
    let message_length: usize = a
        .next()
        .expect("usage: bench-aes <message_length> <num_iterations>")
        .parse()
        .expect("<message_length> must be usize");
    let num_iterations: u32 = a
        .next()
        .expect("usage: bench-aes <message_length> <num_iterations>")
        .parse()
        .expect("<num_iterations> must be u32");
    let args = serde_json::json!({
        "message_length": message_length,
        "num_iterations": num_iterations,
    });
    let out = bench_aes::main(args).expect("bench_aes::main");
    println!("{}", serde_json::to_string(&out).unwrap());
}
