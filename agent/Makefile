.PHONY: build run test clean

build:
	cargo build --release

run:
	cargo run -- --config agent.yaml

test:
	cargo test

clean:
	cargo clean
	rm -rf target
