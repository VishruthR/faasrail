use serde_json::{json, Value};
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{
    Fields, IncomingRequest, Method, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use wasi::io::streams::StreamError;

fn parse_cli(args: &[String]) -> Value {
    let byte_size: usize = args
        .first()
        .expect("usage: bench-disk-seq <byte_size> <file_size_mib>")
        .parse()
        .expect("<byte_size> must be usize");
    let file_size: usize = args
        .get(1)
        .expect("usage: bench-disk-seq <byte_size> <file_size_mib>")
        .parse()
        .expect("<file_size_mib> must be usize");
    json!({ "byte_size": byte_size, "file_size": file_size })
}

fn invoke(args: Value) -> Result<Value, serde_json::Error> {
    bench_disk_seq::main(args)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let input = parse_cli(&args);
    match invoke(input) {
        Ok(v) => println!("{}", serde_json::to_string(&v).unwrap()),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

struct Component;

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let response = match handle_request(request) {
            Ok(body_json) => build_response(200, body_json),
            Err((status, msg)) => build_response(status, json!({ "error": msg })),
        };
        ResponseOutparam::set(response_out, Ok(response));
    }
}

fn handle_request(request: IncomingRequest) -> Result<Value, (u16, String)> {
    if !matches!(request.method(), Method::Post) {
        return Err((405, "method not allowed; use POST".into()));
    }
    let body = request
        .consume()
        .map_err(|_| (400, "could not consume request body".to_string()))?;
    let bytes = read_all(&body).map_err(|e| (400, format!("read body: {e:?}")))?;
    let input: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).map_err(|e| (400, format!("invalid JSON: {e}")))?
    };
    invoke(input).map_err(|e| (500, format!("bench error: {e}")))
}

fn read_all(body: &wasi::http::types::IncomingBody) -> Result<Vec<u8>, StreamError> {
    let stream = body.stream().expect("incoming body stream");
    let mut buf = Vec::new();
    loop {
        match stream.blocking_read(8192) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                buf.extend_from_slice(&chunk);
            }
            Err(StreamError::Closed) => break,
            Err(e) => return Err(e),
        }
    }
    Ok(buf)
}

fn build_response(status: u16, body: Value) -> OutgoingResponse {
    let headers = Fields::new();
    headers
        .set(&"content-type".to_string(), &[b"application/json".to_vec()])
        .expect("set content-type");
    let response = OutgoingResponse::new(headers);
    response.set_status_code(status).expect("set status");
    let out_body = response.body().expect("outgoing body");
    {
        let stream = out_body.write().expect("body write stream");
        let serialized = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
        for chunk in serialized.chunks(4096) {
            stream.blocking_write_and_flush(chunk).expect("write body");
        }
    }
    OutgoingBody::finish(out_body, None).expect("finish body");
    response
}

wasi::http::proxy::export!(Component with_types_in wasi);
