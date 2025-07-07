# Release

## Dependencies

- `pip install flatpak-cargo-generator`

## Process

Make sure it builds and tests pass

```sh
cargo test
```

Bump version in Cargo.toml (hand edit)

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

OStree commit screenshots.

```
ostree commit --repo=repo --canonical-permissions --branch=screenshots/$(flatpak --default-arch) packaging/flatpak/builddir/files/share/app-info/media
```

