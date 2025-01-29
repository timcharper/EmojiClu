RUST_SOURCES := $(shell find src -name "*.rs")
RESOURCE_FILES := $(shell find resources -type f)

.PHONY: linux windows

linux: bundle/gwatson-linux-x86_64.tar.xz

windows: bundle/gwatson-installer.exe

target/release/gwatson: $(RUST_SOURCES) $(RESOURCE_FILES)
	cargo build --release

bundle/gwatson-linux-x86_64.tar.xz: target/release/gwatson
	cd target/release && tar c gwatson | xz -7 -T 0 | pv  > ../../bundle/gwatson-linux-x86_64.tar.xz

bundle/gwatson/bin/gwatson.exe: $(RUST_SOURCES) $(RESOURCE_FILES) resources/gwatson-icon.ico
	./build-windows.sh && ./package-windows.sh

bundle/gwatson-installer.exe: bundle/gwatson/bin/gwatson.exe resources/gwatson-icon.ico
	cp resources/gwatson-icon.ico bundle/gwatson/icon.ico
	makensis ./bundle/installer.nsi

resources/gwatson-icon.ico: resources/gwatson-icon.png
	convert $< -define icon:auto-resize=64,48,32,16 $@