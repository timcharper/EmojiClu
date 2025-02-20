SHELL := /bin/bash

RUST_SOURCES := $(shell find src -name "*.rs")
RESOURCE_FILES := $(shell find resources -type f)

VERSION := $(shell cat version.txt)

.PHONY: linux windows deb flatpak clean clean-packaging

clean-packaging:
	rm -f packaging/*.deb packaging/*.exe packaging/*.tar.xz packaging/*.flatpak
	rm -f packaging/mindhunt-deb/usr/bin/mindhunt
	rm -f packaging/windows/mindhunt/bin/mindhunt.exe
	rm -rf packaging/flatpak/repo
	rm -rf packaging/flatpak/builder
	rm -rf packaging/mindhunt-deb/usr/share/icons/hicolor

clean: clean-packaging
	cargo clean

linux: packaging/mindhunt-linux-$(VERSION)-x86_64.tar.xz

flatpak: target/release/mindhunt
	flatpak-builder --user --install --user --force-clean packaging/flatpak/builder packaging/flatpak/config/org.timcharper.MindHunt.yml
	flatpak build-export packaging/flatpak/repo packaging/flatpak/builder master
	flatpak remote-add --if-not-exists --user mindhunt-local packaging/flatpak/repo --no-gpg-verify
	flatpak build-bundle packaging/flatpak/repo packaging/mindhunt-$(VERSION).flatpak org.timcharper.MindHunt master

windows: packaging/mindhunt-installer-$(VERSION).exe

packaging/windows/installer.nsi: version.txt
	sed -i 's/!define APPVERSION .*/!define APPVERSION $(VERSION)/' $@
	sed -i 's/!define OUTFILE .*/!define OUTFILE mindhunt-installer-$(VERSION).exe/' $@

packaging/mindhunt-deb/DEBIAN/control: version.txt
	sed -i 's/Version: .*/Version: $(shell cat version.txt)/' $@

deb: target/release/mindhunt packaging/mindhunt-deb/DEBIAN/control
	# Copy the executable
	mkdir -p ./packaging/mindhunt-deb/usr/bin \
		./packaging/mindhunt-deb/usr/share/applications \
		./packaging/mindhunt-deb/usr/share/icons/hicolor/{16x16,24x24,32x32,48x48,64x64,128x128,256x256,512x512}/apps
	cp target/release/mindhunt ./packaging/mindhunt-deb/usr/bin/mindhunt

	# Copy icons

	cp target/release/resources/icons/hicolor/24x24/apps/org.timcharper.MindHunt.png ./packaging/mindhunt-deb/usr/share/icons/hicolor/24x24/apps/
	cp target/release/resources/icons/hicolor/32x32/apps/org.timcharper.MindHunt.png ./packaging/mindhunt-deb/usr/share/icons/hicolor/32x32/apps/
	cp target/release/resources/icons/hicolor/48x48/apps/org.timcharper.MindHunt.png ./packaging/mindhunt-deb/usr/share/icons/hicolor/48x48/apps/
	cp target/release/resources/icons/hicolor/64x64/apps/org.timcharper.MindHunt.png ./packaging/mindhunt-deb/usr/share/icons/hicolor/64x64/apps/
	cp target/release/resources/icons/hicolor/128x128/apps/org.timcharper.MindHunt.png ./packaging/mindhunt-deb/usr/share/icons/hicolor/128x128/apps/
	cp target/release/resources/icons/hicolor/256x256/apps/org.timcharper.MindHunt.png ./packaging/mindhunt-deb/usr/share/icons/hicolor/256x256/apps/
	cp target/release/resources/icons/hicolor/512x512/apps/org.timcharper.MindHunt.png ./packaging/mindhunt-deb/usr/share/icons/hicolor/512x512/apps/

	dpkg-deb --build ./packaging/mindhunt-deb ./packaging/mindhunt_$(shell cat version.txt)_amd64.deb




target/release/mindhunt: $(RUST_SOURCES) $(RESOURCE_FILES)
	cargo build --release

packaging/mindhunt-linux-$(VERSION)-x86_64.tar.xz: target/release/mindhunt
	cd target/release && tar c mindhunt | xz -7 -T 0 | pv  > ../../$@

target/x86_64-pc-windows-gnu/release/mindhunt.exe: $(RUST_SOURCES) $(RESOURCE_FILES) resources/mindhunt-icon.ico
	./packaging/windows/build-windows.sh

packaging/windows/mindhunt/bin/mindhunt.exe: target/x86_64-pc-windows-gnu/release/mindhunt.exe
	./packaging/windows/package-windows.sh

packaging/mindhunt-installer-$(VERSION).exe: packaging/windows/mindhunt/bin/mindhunt.exe resources/mindhunt-icon.ico packaging/windows/installer.nsi
	cp resources/mindhunt-icon.ico packaging/windows/mindhunt/icon.ico
	makensis ./packaging/windows/installer.nsi
	mv packaging/windows/mindhunt-installer-$(VERSION).exe $@

resources/mindhunt-icon.ico: resources/mindhunt-icon.png
	convert $< -define icon:auto-resize=64,48,32,16 $@
