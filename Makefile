
VERSION=$(shell cat Cargo.toml | grep version | head -1 | cut -d'"' -f 2)
PWD=$(shell pwd)

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

dist:
	@@echo Build OSX distribution...
	make release
	cd target/release && rm -f reframe_v$(VERSION)-x86_64-darwin.zip && zip -r reframe_v$(VERSION)-x86_64-darwin.zip reframe
	@@echo Build Linux distribution...
	make build-linux-musl
	cd target/x86_64-unknown-linux-musl/release && rm -f reframe_v$(VERSION)-x86_64-linux.zip && zip -r reframe_v$(VERSION)-x86_64-linux.zip reframe


fmt:
	@@cargo fmt

test:
	@@cargo test

clean:
	@@cargo clean

.PHONY: fmt release build-linux-musl clean test dist

