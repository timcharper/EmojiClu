SHELL := /bin/bash

RUST_SOURCES := $(shell find src -name "*.rs")
RESOURCE_FILES := $(shell find resources -type f)

VERSION := $(shell cat version.txt)

.PHONY: linux windows deb flatpak clean clean-packaging

clean-packaging:
	rm -rf artifacts/
	rm -rf packaging/emojiclu-deb/usr/bin/emojiclu
	rm -f packaging/windows/emojiclu/bin/emojiclu.exe
	rm -rf packaging/flatpak/repo
	rm -rf packaging/flatpak/builder
	rm -rf packaging/emojiclu-deb/usr/share/icons/hicolor

clean: clean-packaging
	cargo clean

linux: artifacts/${VERSION}/emojiclu-linux-$(VERSION)-x86_64.tar.xz

flatpak: artifacts/${VERSION}/emojiclu-$(VERSION).flatpak

artifacts/${VERSION}/emojiclu-$(VERSION).flatpak: target/release/emojiclu
	mkdir -p artifacts/${VERSION}
	flatpak-builder --user --install --user --force-clean packaging/flatpak/builder packaging/flatpak/config/org.timcharper.EmojiClu.yml
	flatpak build-export packaging/flatpak/repo packaging/flatpak/builder master
	flatpak remote-add --if-not-exists --user emojiclu-local packaging/flatpak/repo --no-gpg-verify
	flatpak build-bundle packaging/flatpak/repo $@ org.timcharper.EmojiClu master

windows: artifacts/${VERSION}/emojiclu-installer-$(VERSION).exe

packaging/windows/installer.nsi: version.txt
	sed -i 's/!define APPVERSION .*/!define APPVERSION $(VERSION)/' $@
	sed -i 's/!define OUTFILE .*/!define OUTFILE emojiclu-installer-$(VERSION).exe/' $@

packaging/emojiclu-deb/DEBIAN/control: version.txt
	sed -i 's/Version: .*/Version: $(shell cat version.txt)/' $@

deb: artifacts/${VERSION}/emojiclu_${VERSION}_amd64.deb

artifacts/${VERSION}/emojiclu_${VERSION}_amd64.deb: target/release/emojiclu packaging/emojiclu-deb/DEBIAN/control
	mkdir -p artifacts/${VERSION}
	mkdir -p ./packaging/emojiclu-deb/usr/bin \
		./packaging/emojiclu-deb/usr/share/applications \
		./packaging/emojiclu-deb/usr/share/icons/hicolor/{16x16,24x24,32x32,48x48,64x64,128x128,256x256,512x512}/apps
	cp target/release/emojiclu ./packaging/emojiclu-deb/usr/bin/emojiclu

	# Copy icons
	cp target/release/resources/icons/hicolor/24x24/apps/org.timcharper.EmojiClu.png ./packaging/emojiclu-deb/usr/share/icons/hicolor/24x24/apps/
	cp target/release/resources/icons/hicolor/32x32/apps/org.timcharper.EmojiClu.png ./packaging/emojiclu-deb/usr/share/icons/hicolor/32x32/apps/
	cp target/release/resources/icons/hicolor/48x48/apps/org.timcharper.EmojiClu.png ./packaging/emojiclu-deb/usr/share/icons/hicolor/48x48/apps/
	cp target/release/resources/icons/hicolor/64x64/apps/org.timcharper.EmojiClu.png ./packaging/emojiclu-deb/usr/share/icons/hicolor/64x64/apps/
	cp target/release/resources/icons/hicolor/128x128/apps/org.timcharper.EmojiClu.png ./packaging/emojiclu-deb/usr/share/icons/hicolor/128x128/apps/
	cp target/release/resources/icons/hicolor/256x256/apps/org.timcharper.EmojiClu.png ./packaging/emojiclu-deb/usr/share/icons/hicolor/256x256/apps/
	cp target/release/resources/icons/hicolor/512x512/apps/org.timcharper.EmojiClu.png ./packaging/emojiclu-deb/usr/share/icons/hicolor/512x512/apps/

	fakeroot dpkg-deb --build ./packaging/emojiclu-deb $@

target/release/emojiclu: $(RUST_SOURCES) $(RESOURCE_FILES)
	cargo build --release

artifacts/${VERSION}/emojiclu-linux-$(VERSION)-x86_64.tar.xz: target/release/emojiclu
	mkdir -p artifacts/${VERSION}
	cd target/release && tar c emojiclu | xz -7 -T 0 | pv  > ../../$@

target/x86_64-pc-windows-gnu/release/emojiclu.exe: $(RUST_SOURCES) $(RESOURCE_FILES) resources/emojiclu-icon.ico
	./packaging/windows/build-windows.sh

packaging/windows/emojiclu/bin/emojiclu.exe: target/x86_64-pc-windows-gnu/release/emojiclu.exe
	./packaging/windows/package-windows.sh

artifacts/${VERSION}/emojiclu-installer-$(VERSION).exe: packaging/windows/emojiclu/bin/emojiclu.exe resources/emojiclu-icon.ico packaging/windows/installer.nsi
	mkdir -p artifacts/${VERSION}
	cp resources/emojiclu-icon.ico packaging/windows/emojiclu/icon.ico
	makensis ./packaging/windows/installer.nsi
	mv packaging/windows/emojiclu-installer-$(VERSION).exe $@

resources/emojiclu-icon.ico: resources/emojiclu-icon.png
	convert $< -define icon:auto-resize=64,48,32,16 $@
