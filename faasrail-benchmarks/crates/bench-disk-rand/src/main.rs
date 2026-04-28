use serde_json::Value;

static mut MEMORY_BUFFER: Vec<u8> = Vec::new();

#[no_mangle]
pub fn get_wasm_memory_buffer_pointer() -> *mut u8 {
    unsafe { MEMORY_BUFFER.as_mut_ptr() }
}

#[no_mangle]
pub fn wasm_memory_buffer_allocate_space(num_elems: usize) {
    unsafe {
        MEMORY_BUFFER.reserve(num_elems);
        MEMORY_BUFFER.set_len(num_elems);
    }
}

#[no_mangle]
pub fn get_wasm_memory_buffer_len() -> usize {
    unsafe { MEMORY_BUFFER.len() }
}

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let len: usize = args[args.len() - 1].parse().unwrap();
    let slice = unsafe { &MEMORY_BUFFER[..len] };
    let json: Value = serde_json::from_slice(slice).expect("deserialize failed");

    let result = bench_disk_rand::main(json);

    let wrapped = match result {
        Ok(v) => serde_json::json!({"Ok": v}),
        Err(e) => serde_json::json!({"Err": e.to_string()}),
    };

    let mut out = serde_json::to_vec(&wrapped).unwrap();
    unsafe {
        MEMORY_BUFFER.clear();
        MEMORY_BUFFER.append(&mut out);
    }
}
