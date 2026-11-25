# Senzu SDK. `just check` runs everything CI does.

msrv := "1.88.0"

# Format, lint, test, docs, MSRV, wasm build, and the vectors in sync with the contracts
check: fmt-check lint test doc msrv wasm vectors-check

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo clippy -p senzu-sdk-core --features wasm --target wasm32-unknown-unknown -- -D warnings

test:
    cargo test --workspace --all-features

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

msrv:
    cargo +{{msrv}} check -p senzu-sdk-core --all-features

wasm:
    cargo build -p senzu-sdk-core --features wasm --target wasm32-unknown-unknown --release

# The TypeScript core package (ts-sdk/core/pkg), rebuilt from the Rust core and committed
wasm-pkg:
    cd ts-sdk/core && npm run build

# The vectors here are the ones the contracts generate (contracts/formal/vectors, the senzu-fi/senzu-core submodule)
vectors-check:
    diff -r contracts/formal/vectors rust-sdk/core/tests/vectors
