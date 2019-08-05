
PWD = $(shell pwd)

all: release build-linux-musl

release:
	@@echo "building..."
	@@cargo build --release

build-linux-musl:
	@@echo "building using docker..."
	@@docker run -it --rm -v $(PWD):/workdir \
		-v /tmp:/root/.cargo/git \
		-v /tmp:/root/.cargo/registry \
		anvie/rust-musl-build:rust_nightly \
		make _build-linux-musl

_build-linux-musl:
	cargo update
	cargo build --release --target=x86_64-unknown-linux-musl

fmt:
	@@cargo fmt

test:
	@@cargo test

clean:
	@@cargo clean

.PHONY: fmt release build-linux-musl clean test

