BINARY := wayshot
BUILDFLAGS := --release
TARGET_DIR := /usr/bin
SOURCE_DIR := ./target/release

all: build

build:
	@cargo build $(BUILDFLAGS)

run:
	@cargo run

install:
	@mkdir -p $(TARGET_DIR)
	@cp $(SOURCE_DIR)/$(BINARY) $(TARGET_DIR)
	@chmod +x $(TARGET_DIR)/$(BINARY)

uninstall:
	@rm $(TARGET_DIR)/$(BINARY)

check:
	@cargo fmt
	@cargo check

clean:
	@cargo clean

setup:
	@rustup install stable
	@rustup default stable

.PHONY: check clean setup all install build
