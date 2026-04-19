extern crate bench_chameleon;
extern crate serde_json;

fn main() {
    let mut a = std::env::args().skip(1);
    let cols: usize = a
        .next()
        .expect("usage: bench-chameleon <num_of_cols> <num_of_rows>")
        .parse()
        .expect("<num_of_cols> must be usize");
    let rows: usize = a
        .next()
        .expect("usage: bench-chameleon <num_of_cols> <num_of_rows>")
        .parse()
        .expect("<num_of_rows> must be usize");
    let args = serde_json::json!({
        "num_of_cols": cols,
        "num_of_rows": rows,
    });
    let out = bench_chameleon::main(args).expect("bench_chameleon::main");
    println!("{}", serde_json::to_string(&out).unwrap());
}
