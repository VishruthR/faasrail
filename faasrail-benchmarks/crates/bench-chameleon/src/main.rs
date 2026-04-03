use bench_common::{black_box, write_output, Timer};
use serde::Serialize;
use std::env;

#[derive(Serialize)]
struct Output {
    html_length: usize,
    elapsed_ms: f64,
}

fn render_table(rows: usize, cols: usize) -> String {
    let mut html = String::with_capacity(rows * cols * 40);
    html.push_str("<table>\n");
    for r in 0..rows {
        html.push_str("  <tr>\n");
        for c in 0..cols {
            html.push_str("    <td>");
            // Alternate between string and numeric content like the original benchmark
            if c % 2 == 0 {
                html.push_str(&format!("Row {}, Col {}", r, c));
            } else {
                html.push_str(&format!("{}", r * cols + c));
            }
            html.push_str("</td>\n");
        }
        html.push_str("  </tr>\n");
    }
    html.push_str("</table>");
    html
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let num_of_cols = args[1].parse::<usize>().unwrap();
    let num_of_rows = args[2].parse::<usize>().unwrap();
    let timer = Timer::start();
    let html = render_table(num_of_rows, num_of_cols);
    let len = black_box(html.len());
    let elapsed_ms = timer.elapsed_ms();
    write_output(&Output { html_length: len, elapsed_ms });
}
