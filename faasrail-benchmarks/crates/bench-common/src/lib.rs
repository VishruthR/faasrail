use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Instant;

/// Read input from CLI args and deserialize into `T`.
///
/// `fields` must list the struct field names in alphabetical order.
/// Each CLI arg is matched positionally to the corresponding field.
/// Values are auto-detected as integers, floats, or strings.
pub fn read_input<T: DeserializeOwned>(fields: &[&str]) -> T {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(
        args.len(),
        fields.len(),
        "expected {} args ({}), got {}",
        fields.len(),
        fields.join(", "),
        args.len()
    );

    let map: serde_json::Map<String, serde_json::Value> = fields
        .iter()
        .zip(args.iter())
        .map(|(field, value)| {
            let json_value = if let Ok(n) = value.parse::<u64>() {
                serde_json::Value::Number(n.into())
            } else if let Ok(f) = value.parse::<f64>() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(value.clone()))
            } else {
                serde_json::Value::String(value.clone())
            };
            (field.to_string(), json_value)
        })
        .collect();

    serde_json::from_value(serde_json::Value::Object(map)).expect("failed to deserialize args")
}

/// Serialize `value` as JSON and write to stdout.
pub fn write_output<T: Serialize>(value: &T) {
    let out = serde_json::to_string(value).expect("failed to serialize output");
    println!("{out}");
}

/// Simple timer wrapping `std::time::Instant`.
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn start() -> Self {
        Self { start: Instant::now() }
    }

    /// Returns elapsed time in milliseconds as f64.
    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}
