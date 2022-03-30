fmt:
	cargo fmt

lint: fmt
	cargo clippy --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check

# Run unit tests.
test:
	cargo test -- --nocapture $(run)
