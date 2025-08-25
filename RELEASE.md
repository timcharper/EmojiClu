# Release

## Dependencies

- `pip install flatpak-cargo-generator`
- `cargo install cargo-edit`

## Process

Make sure it builds and tests pass

```sh
cargo test
```

Bump version in Cargo.toml

```sh
version=1.1.1
cargo set-version "$version"
```

Update artifacts, commit, tag, push:

```sh
make tag
```

Build the assets

```sh
make windows linux deb
```

Create new release, upload artifacts, etc.

Bump flatpak:

```sh
make bump-flatpak
```

Push github release:

```sh
gh release create v${version} --generate-notes ./artifacts/${version}/*
```

Build flatpak

```sh
flatpak run org.flatpak.Builder \
  --force-clean \
  --sandbox \
  --user \
  --install \
  --install-deps-from=flathub \
  --ccache \
  --mirror-screenshots-url=https://dl.flathub.org/media/ \
  --repo=./packaging/flatpak/repo \
  ./packaging/flatpak/builddir \
  io.github.timcharper.EmojiClu.yml
```

Update flathub (https://github.com/flathub/io.github.timcharper.EmojiClu):

```sh
cp cargo-sources.json ../io.github.timcharper.EmojiClu/
cp io.github.timcharper.EmojiClu.yml ../io.github.timcharper.EmojiClu/
cd ../io.github.timcharper.EmojiClu/
VERSION_TAG="$(yq e '.modules[0].sources[0].tag' io.github.timcharper.EmojiClu.yml)"
git checkout -b release/${VERSION_TAG}
git add .
git commit -m "release emojiClu ${VERSION_TAG}"
git push
gh pr create --title "Relase EmojiClu $VERSION_TAG"
```
