BINARY := wayshot
BUILDFLAGS := --release
TARGET_DIR := /usr/bin

all: build

build:
	@cargo build $(BUILDFLAGS) --target=x86_64-unknown-linux-musl
	@cp ./target/x86_64-unknown-linux-musl/release/$(BINARY) ./bin/$(BINARY)

glibc:
	@cargo build $(BUILDFLAGS)
	@cp ./target/release/$(BINARY) ./bin/$(BINARY)

install:
	@mkdir -p $(TARGET_DIR)
	@cp ./bin/$(BINARY) $(TARGET_DIR)
	@chmod +x $(TARGET_DIR)/$(BINARY)

uninstall:
	@rm $(TARGET_DIR)/$(BINARY)

check:
	@cargo fmt
	@cargo check --target=x86_64-unknown-linux-musl

clean:
	@cargo clean

setup:
	@mkdir -p ./bin
	@rustup install stable
	@rustup default stable
	@rustup target add x86_64-unknown-linux-musl

.PHONY: check clean setup all install build glibc
