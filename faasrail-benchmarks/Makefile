TARGET = wasm32-wasip1
RELEASE_DIR = target/$(TARGET)/release

.PHONY: build clean

build:
	cargo build --release --target $(TARGET)

clean:
	cargo clean

# Quick smoke tests (requires wasmtime)
.PHONY: test-float test-json test-chameleon test-aes test-gzip test-disk-seq test-disk-rand test-all

test-float: build
	echo '{"n":10000}' | wasmtime run $(RELEASE_DIR)/bench-float.wasm

test-json: build
	echo '{"json_string":"{\"a\":1,\"b\":[1,2,3]}"}' | wasmtime run $(RELEASE_DIR)/bench-json.wasm

test-chameleon: build
	echo '{"num_of_rows":100,"num_of_cols":10}' | wasmtime run $(RELEASE_DIR)/bench-chameleon.wasm

test-aes: build
	echo '{"message_length":256,"num_iterations":10}' | wasmtime run $(RELEASE_DIR)/bench-aes.wasm

test-gzip: build
	echo '{"file_size":1}' | wasmtime run --dir=/tmp $(RELEASE_DIR)/bench-gzip.wasm

test-disk-seq: build
	echo '{"file_size":1,"byte_size":4096}' | wasmtime run --dir=/tmp $(RELEASE_DIR)/bench-disk-seq.wasm

test-disk-rand: build
	echo '{"file_size":1,"byte_size":4096}' | wasmtime run --dir=/tmp $(RELEASE_DIR)/bench-disk-rand.wasm

test-all: test-float test-json test-chameleon test-aes test-gzip test-disk-seq test-disk-rand
