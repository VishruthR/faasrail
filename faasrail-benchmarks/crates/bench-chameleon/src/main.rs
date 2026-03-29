use bench_common::{read_input, write_output, Timer};
use serde::{Deserialize, Serialize};
use std::hint::black_box;

#[derive(Deserialize)]
struct Input {
    num_of_rows: usize,
    num_of_cols: usize,
}

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
                html.push_str(&format!("Row {r}, Col {c}"));
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
    let input: Input = read_input();
    let timer = Timer::start();
    let html = render_table(input.num_of_rows, input.num_of_cols);
    let len = black_box(html.len());
    let elapsed_ms = timer.elapsed_ms();
    write_output(&Output { html_length: len, elapsed_ms });
}
