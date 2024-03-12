BINARY := wayshot
BUILDFLAGS := --release
TARGET_DIR := /usr/bin
SOURCE_DIR := ./target/release
MAN1_DIR := /usr/share/man/man1
MAN7_DIR := /usr/share/man/man7

all: build

build:
	@cargo build $(BUILDFLAGS)

run:
	@cargo run

docs:
	@echo -n 'Generating docs with scdoc and gzip ... '
	@for file in ./docs/*.scd ; do \
		scdoc < "$$file" | gzip --best > "$${file%.scd}.gz" ; \
	done
	@echo 'done!'

install: build docs
	@mkdir -pv $(TARGET_DIR)
	@cp -v $(SOURCE_DIR)/$(BINARY) $(TARGET_DIR)
	@chmod +x $(TARGET_DIR)/$(BINARY)
	@cp -v ./docs/wayshot.1.gz $(MAN1_DIR)
	@cp -v ./docs/wayshot.7.gz $(MAN7_DIR)


uninstall:
	@rm -fv $(TARGET_DIR)/$(BINARY)
	@rm -fv /usr/share/man/**/wayshot.*

check:
	@cargo fmt
	@cargo check
	@cargo clippy

clean:
	@cargo clean
	@rm -fv ./docs/*.gz

setup:
	@rustup install stable
	@rustup default stable

.PHONY: check clean setup all install build docs
