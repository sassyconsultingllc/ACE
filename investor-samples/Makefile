# Sassy Browser Build System
# Makefile demo

.PHONY: all build release clean test

CARGO = cargo
VERSION = 2.0.0
TARGET = target/release/sassy-browser

all: build

build:
	$(CARGO) build

release:
	$(CARGO) build --release
	@echo "Built $(TARGET)"

clean:
	$(CARGO) clean
	rm -rf dist/

test:
	$(CARGO) test

check:
	$(CARGO) check
	$(CARGO) clippy

package: release
	mkdir -p dist
	cp $(TARGET) dist/
	cp -r assets dist/
	tar -czvf sassy-browser-$(VERSION).tar.gz dist/
