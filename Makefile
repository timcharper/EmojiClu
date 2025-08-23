SHELL := /bin/bash

RUST_SOURCES := $(shell find src -name "*.rs")
RESOURCE_FILES := $(shell find resources -type f)

# Define common paths for resources
DESKTOP_SOURCE := resources/io.github.timcharper.EmojiClu.desktop

VERSION := $(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')

.PHONY: linux windows deb flatpak clean clean-packaging tag packaging/dist bump-flatpak

tag: packaging/windows/installer.nsi packaging/emojiclu-deb/DEBIAN/control
	cargo test
	git status || git commit -a -m "Bump version to $(VERSION)"
	git tag -a v$(VERSION) -m "Release $(VERSION)"
	git push origin v$(VERSION) main

bump-flatpak:
	TAG="v$(VERSION)" COMMIT="$(shell git rev-parse v$(VERSION))" yq -i '.modules[0].sources[] |= (select(.type == "git") | .tag = strenv(TAG) | .commit = strenv(COMMIT))' io.github.timcharper.EmojiClu.yml
	# update appdata.xml
	date=$$(date +%Y-%m-%d); \
	sed -i 's|<release version="[^"]*" date="[^"]*"|<release version="$(VERSION)" date="'"$$date"'"|' packaging/flatpak/metainfo/io.github.timcharper.EmojiClu.appdata.xml
	flatpak-cargo-generator Cargo.lock -o cargo-sources.json
	# run a test build
	flatpak-builder --user --install --user --force-clean ./packaging/flatpak/builddir io.github.timcharper.EmojiClu.yml

clean-packaging:
	rm -rf artifacts/
	rm -rf packaging/dist
	rm -rf packaging/emojiclu-deb/usrt l
	rm -f packaging/windows/emojiclu/bin/emojiclu.exe
	rm -rf packaging/flatpak/repo
	rm -rf packaging/flatpak/builddir

clean: clean-packaging
	cargo clean

linux: artifacts/${VERSION}/emojiclu-linux-$(VERSION)-x86_64.tar.xz

cargo-sources.json: Cargo.lock
	flatpak-cargo-generator Cargo.lock -o $@

windows: artifacts/${VERSION}/emojiclu-installer-$(VERSION).exe

packaging/windows/installer.nsi: Cargo.toml
	sed -i 's/!define APPVERSION .*/!define APPVERSION $(VERSION)/' $@
	sed -i 's/!define OUTFILE .*/!define OUTFILE emojiclu-installer-$(VERSION).exe/' $@

packaging/emojiclu-deb/DEBIAN/control: Cargo.toml
	sed -i 's/Version: .*/Version: $(VERSION)/' $@

# New target to assemble all installable artifacts into packaging/dist
packaging/dist: target/release/emojiclu $(DESKTOP_SOURCE)
	@echo "Assembling artifacts into packaging/dist..."
	rm -rf $@ # Clean previous dist build
	mkdir -p $@/bin \
	         $@/share/applications \
	         $@/share/icons/hicolor/{16x16,24x24,32x32,48x48,64x64,128x128,256x256,512x512}/apps
	cp target/release/emojiclu $@/bin/emojiclu
	cp $(DESKTOP_SOURCE) $@/share/applications/
	cp target/release/resources/icons/hicolor/16x16/apps/io.github.timcharper.EmojiClu.png $@/share/icons/hicolor/16x16/apps/
	cp target/release/resources/icons/hicolor/24x24/apps/io.github.timcharper.EmojiClu.png $@/share/icons/hicolor/24x24/apps/
	cp target/release/resources/icons/hicolor/32x32/apps/io.github.timcharper.EmojiClu.png $@/share/icons/hicolor/32x32/apps/
	cp target/release/resources/icons/hicolor/48x48/apps/io.github.timcharper.EmojiClu.png $@/share/icons/hicolor/48x48/apps/
	cp target/release/resources/icons/hicolor/64x64/apps/io.github.timcharper.EmojiClu.png $@/share/icons/hicolor/64x64/apps/
	cp target/release/resources/icons/hicolor/128x128/apps/io.github.timcharper.EmojiClu.png $@/share/icons/hicolor/128x128/apps/
	cp target/release/resources/icons/hicolor/256x256/apps/io.github.timcharper.EmojiClu.png $@/share/icons/hicolor/256x256/apps/
	cp target/release/resources/icons/hicolor/512x512/apps/io.github.timcharper.EmojiClu.png $@/share/icons/hicolor/512x512/apps/

deb: artifacts/${VERSION}/emojiclu_${VERSION}_amd64.deb

artifacts/${VERSION}/emojiclu_${VERSION}_amd64.deb: packaging/dist packaging/emojiclu-deb/DEBIAN/control
	@echo "Building Debian package..."
	mkdir -p artifacts/${VERSION}
	# Clean the debian build directory's contents (excluding DEBIAN/)
	rm -rf ./packaging/emojiclu-deb/usr
	# Copy all assembled artifacts from packaging/dist to the debian build directory
	rsync -av packaging/dist/ ./packaging/emojiclu-deb/usr/ --delete

	fakeroot dpkg-deb --build ./packaging/emojiclu-deb $@

target/release/emojiclu: $(RUST_SOURCES) $(RESOURCE_FILES)
	cargo build --release $(if $(CARGO_OFFLINE),--offline) --all-features

artifacts/${VERSION}/emojiclu-linux-$(VERSION)-x86_64.tar.xz: target/release/emojiclu
	mkdir -p artifacts/${VERSION}
	cd target/release && tar c emojiclu | xz -7 -T 0 | pv > ../../$@

target/x86_64-pc-windows-gnu/release/emojiclu.exe: $(RUST_SOURCES) $(RESOURCE_FILES) packaging/windows/emojiclu/icon.ico
	./packaging/windows/build-windows.sh

packaging/windows/emojiclu/bin/emojiclu.exe: target/x86_64-pc-windows-gnu/release/emojiclu.exe
	./packaging/windows/package-windows.sh

packaging/windows/emojiclu/icon.ico: resources/emojiclu-icon.png
	convert $< -define icon:auto-resize=64,48,32,16 $@

artifacts/${VERSION}/emojiclu-installer-$(VERSION).exe: packaging/windows/emojiclu/bin/emojiclu.exe packaging/windows/emojiclu/icon.ico packaging/windows/installer.nsi
	mkdir -p artifacts/${VERSION}
	makensis ./packaging/windows/installer.nsi
	mv packaging/windows/emojiclu-installer-$(VERSION).exe $@