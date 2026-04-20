TARGET = wasm32-wasip2
RELEASE_DIR = target/$(TARGET)/release
WASMTIME_FLAGS = -Shttp -Scli
SERVE_FLAGS = -Scli --dir=.::.

.PHONY: build clean
build:
	cargo build --release --target $(TARGET)

clean:
	cargo clean

# ── Quick smoke tests via `wasmtime run` (single-shot CLI) ────────────────────
.PHONY: run-float run-json run-chameleon run-aes run-gzip run-disk-seq run-disk-rand run-all

run-float: build
	wasmtime run $(WASMTIME_FLAGS) $(RELEASE_DIR)/bench-float.wasm 10000

run-json: build
	wasmtime run $(WASMTIME_FLAGS) $(RELEASE_DIR)/bench-json.wasm '{"a":1,"b":[1,2,3]}'

run-chameleon: build
	wasmtime run $(WASMTIME_FLAGS) $(RELEASE_DIR)/bench-chameleon.wasm 10 100

run-aes: build
	wasmtime run $(WASMTIME_FLAGS) $(RELEASE_DIR)/bench-aes.wasm 256 10

run-gzip: build
	wasmtime run $(WASMTIME_FLAGS) --dir=.::. $(RELEASE_DIR)/bench-gzip.wasm 1

run-disk-seq: build
	wasmtime run $(WASMTIME_FLAGS) --dir=.::. $(RELEASE_DIR)/bench-disk-seq.wasm 4096 1

run-disk-rand: build
	wasmtime run $(WASMTIME_FLAGS) --dir=.::. $(RELEASE_DIR)/bench-disk-rand.wasm 4096 1

run-all: run-float run-json run-chameleon run-aes run-gzip run-disk-seq run-disk-rand
