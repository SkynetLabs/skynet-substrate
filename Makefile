fmt:
	cargo fmt

lint: fmt
	cargo clippy --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check

# Run unit tests.
test:
	cargo test -- --nocapture $(run)

doc:
    cargo doc --no-deps
    rm -rf ./docs
    echo '<meta http-equiv="refresh" content="0; url=skynet_substrate/">' > target/doc/index.html
    cp -r target/doc ./docs
