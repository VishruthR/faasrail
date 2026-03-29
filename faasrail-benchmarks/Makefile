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
	wasmtime run $(RELEASE_DIR)/bench-float.wasm 10000

test-json: build
	wasmtime run $(RELEASE_DIR)/bench-json.wasm '{"a":1,"b":[1,2,3]}'

test-chameleon: build
	wasmtime run $(RELEASE_DIR)/bench-chameleon.wasm 10 100

test-aes: build
	wasmtime run $(RELEASE_DIR)/bench-aes.wasm 256 10

test-gzip: build
	wasmtime run --dir=.::. $(RELEASE_DIR)/bench-gzip.wasm 1

test-disk-seq: build
	wasmtime run --dir=.::. $(RELEASE_DIR)/bench-disk-seq.wasm 4096 1

test-disk-rand: build
	wasmtime run --dir=.::. $(RELEASE_DIR)/bench-disk-rand.wasm 4096 1

test-all: test-float test-json test-chameleon test-aes test-gzip test-disk-seq test-disk-rand
