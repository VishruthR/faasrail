extern crate serde_json;
extern crate serde_derive;

use serde_derive::{Deserialize, Serialize};
use serde_json::{Error, Value};

#[derive(Deserialize)]
struct Input {
    num_of_cols: usize,
    num_of_rows: usize,
}

#[derive(Serialize)]
struct Output {
    html_length: usize,
    elapsed_ms: f64,
}

fn elapsed_ms(start: std::time::Instant) -> f64 {
    let d = start.elapsed();
    (d.as_secs() as f64) * 1_000.0 + (d.subsec_nanos() as f64) / 1_000_000.0
}

#[inline]
fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = std::ptr::read_volatile(&dummy as *const T);
        std::mem::forget(dummy);
        ret
    }
}

fn render_table(rows: usize, cols: usize) -> String {
    let mut html = String::with_capacity(rows * cols * 40);
    html.push_str("<table>\n");
    for r in 0..rows {
        html.push_str("  <tr>\n");
        for c in 0..cols {
            html.push_str("    <td>");
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

pub fn main(args: Value) -> Result<Value, Error> {
    let input: Input = serde_json::from_value(args)?;
    let start = std::time::Instant::now();
    let html = render_table(input.num_of_rows, input.num_of_cols);
    let html_length = black_box(html.len());
    serde_json::to_value(Output { html_length, elapsed_ms: elapsed_ms(start) })
}
