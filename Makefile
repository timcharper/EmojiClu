RUST_SOURCES := $(shell find src -name "*.rs")
RESOURCE_FILES := $(shell find resources -type f)

.PHONY: linux windows

linux: bundle/mindhunt-linux-x86_64.tar.xz

flatpak: target/release/mindhunt
	flatpak-builder --user --install --force-clean build-dir org.timcharper.MindHunt.yml

windows: bundle/mindhunt-installer.exe

target/release/mindhunt: $(RUST_SOURCES) $(RESOURCE_FILES)
	cargo build --release

bundle/mindhunt-linux-x86_64.tar.xz: target/release/mindhunt
	cd target/release && tar c mindhunt | xz -7 -T 0 | pv  > ../../bundle/mindhunt-linux-x86_64.tar.xz

target/x86_64-pc-windows-gnu/release/mindhunt.exe: $(RUST_SOURCES) $(RESOURCE_FILES) resources/mindhunt-icon.ico
	./build-windows.sh

bundle/mindhunt/bin/mindhunt.exe: target/x86_64-pc-windows-gnu/release/mindhunt.exe
	./package-windows.sh

bundle/mindhunt-installer.exe: bundle/mindhunt/bin/mindhunt.exe resources/mindhunt-icon.ico bundle/installer.nsi
	cp resources/mindhunt-icon.ico bundle/mindhunt/icon.ico
	makensis ./bundle/installer.nsi

resources/mindhunt-icon.ico: resources/mindhunt-icon.png
	convert $< -define icon:auto-resize=64,48,32,16 $@
