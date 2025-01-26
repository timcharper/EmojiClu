RUST_SOURCES := $(shell find src -name "*.rs")
RESOURCE_FILES := $(shell find resources -type f)

.PHONY: linux windows

linux: target/release/gwatson-linux-x86_64.tar.xz

windows: bundle/gwatson-win32.zip

target/release/gwatson: $(RUST_SOURCES) $(RESOURCE_FILES)
	cargo build --release

target/release/gwatson-linux-x86_64.tar.xz: target/release/gwatson
	cd target/release && tar c gwatson | xz -7 -T 0 | pv  > gwatson-linux-x86_64.tar.xz

bundle/gwatson-win32.zip: bundle/gwatson/bin/gwatson.exe
	cd bundle && zip -r gwatson-win32.zip gwatson

bundle/gwatson/bin/gwatson.exe: $(RUST_SOURCES) $(RESOURCE_FILES)
	./build-windows.sh && ./package-windows.sh