
PWD = $(shell pwd)

all: build build-linux-musl

build:
	@@echo "building..."
	@@cargo build

build-linux-musl:
	@@echo "building using docker..."
	@@docker run -it --rm -v $(PWD):/workdir \
		-v /tmp:/root/.cargo/git \
		-v /tmp:/root/.cargo/registry \
		anvie/rust-musl-build:latest \
		cargo build --release --target=x86_64-unknown-linux-musl

fmt:
	@@cargo fmt

.PHONY: fmt build build-linux-musl

