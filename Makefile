SHELL := /bin/bash

RUST_SOURCES := $(shell find src -name "*.rs")
RESOURCE_FILES := $(shell find resources -type f)

.PHONY: linux windows

linux: bundle/mindhunt-linux-x86_64.tar.xz

flatpak: target/release/mindhunt
	flatpak-builder --user --install --force-clean build-dir org.timcharper.MindHunt.yml

windows: bundle/mindhunt-installer.exe

bundle/mindhunt-deb/DEBIAN/control: version.txt
	sed -i 's/Version: .*/Version: $(shell cat version.txt)/' bundle/mindhunt-deb/DEBIAN/control

deb: target/release/mindhunt bundle/mindhunt-deb/DEBIAN/control
	# Copy the executable
	mkdir -p ./bundle/mindhunt-deb/usr/bin \
		./bundle/mindhunt-deb/usr/share/applications \
		./bundle/mindhunt-deb/usr/share/icons/hicolor/{16x16,24x24,32x32,48x48,64x64,128x128,256x256,512x512}/apps
	cp target/release/mindhunt ./bundle/mindhunt-deb/usr/bin/mindhunt

	# Copy icons

	cp target/release/resources/icons/hicolor/24x24/apps/org.timcharper.MindHunt.png ./bundle/mindhunt-deb/usr/share/icons/hicolor/24x24/apps/
	cp target/release/resources/icons/hicolor/32x32/apps/org.timcharper.MindHunt.png ./bundle/mindhunt-deb/usr/share/icons/hicolor/32x32/apps/
	cp target/release/resources/icons/hicolor/48x48/apps/org.timcharper.MindHunt.png ./bundle/mindhunt-deb/usr/share/icons/hicolor/48x48/apps/
	cp target/release/resources/icons/hicolor/64x64/apps/org.timcharper.MindHunt.png ./bundle/mindhunt-deb/usr/share/icons/hicolor/64x64/apps/
	cp target/release/resources/icons/hicolor/128x128/apps/org.timcharper.MindHunt.png ./bundle/mindhunt-deb/usr/share/icons/hicolor/128x128/apps/
	cp target/release/resources/icons/hicolor/256x256/apps/org.timcharper.MindHunt.png ./bundle/mindhunt-deb/usr/share/icons/hicolor/256x256/apps/
	cp target/release/resources/icons/hicolor/512x512/apps/org.timcharper.MindHunt.png ./bundle/mindhunt-deb/usr/share/icons/hicolor/512x512/apps/

	dpkg-deb --build ./bundle/mindhunt-deb ./bundle/mindhunt_$(shell cat version.txt)_amd64.deb




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
