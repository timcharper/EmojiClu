RUST_SOURCES := $(shell find src -name "*.rs")
RESOURCE_FILES := $(shell find resources -type f)

.PHONY: linux windows

linux: bundle/gnomeclu-linux-x86_64.tar.xz

windows: bundle/gnomeclu-installer.exe

target/release/gnomeclu: $(RUST_SOURCES) $(RESOURCE_FILES)
	cargo build --release

bundle/gnomeclu-linux-x86_64.tar.xz: target/release/gnomeclu
	cd target/release && tar c gnomeclu | xz -7 -T 0 | pv  > ../../bundle/gnomeclu-linux-x86_64.tar.xz

bundle/gnomeclu/bin/gnomeclu.exe: $(RUST_SOURCES) $(RESOURCE_FILES) resources/gnomeclu-icon.ico
	./build-windows.sh && ./package-windows.sh

bundle/gnomeclu-installer.exe: bundle/gnomeclu/bin/gnomeclu.exe resources/gnomeclu-icon.ico
	cp resources/gnomeclu-icon.ico bundle/gnomeclu/icon.ico
	makensis ./bundle/installer.nsi

resources/gnomeclu-icon.ico: resources/gnomeclu-icon.png
	convert $< -define icon:auto-resize=64,48,32,16 $@
