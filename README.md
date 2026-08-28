# Elixir Font Discovery

Elixir Font Discovery resolves installed fonts through the host operating
system and returns standalone OpenType or TrueType font bytes.

- Linux uses Fontconfig.
- macOS uses CoreText.
- Windows uses DirectWrite.
- Published releases use verified precompiled NIFs. Applications that depend
  on the library do not need Rust or a native compiler.

## Installation

Add `elixir_font_discovery` to your dependencies:

```elixir
def deps do
  [
    {:elixir_font_discovery, "~> 0.1.0"}
  ]
end
```

## Usage

Resolve the operating system's regular sans-serif face:

```elixir
{:ok, font} = ElixirFontDiscovery.resolve("sans-serif")

font.family
font.weight
font.style
font.data
```

Request the closest installed bold italic face:

```elixir
{:ok, font} = ElixirFontDiscovery.resolve("Inter", 700, :italic)
```

Named families return `{:error, :not_found}` when the operating system cannot
resolve them. Generic names supported by the library are `sans-serif`,
`system-ui`, `serif`, and `monospace`.

## Developing the native adapter

Library users do not need Rust. Maintainers working on the native adapter can
force a local source build:

```sh
mise install
ELIXIR_FONT_DISCOVERY_BUILD=true mix deps.get
ELIXIR_FONT_DISCOVERY_BUILD=true mix test
```

The Rust crate requires Rust 1.77 or newer. Linux source builds also require
Fontconfig and FreeType development headers.

```sh
cargo test --manifest-path native/elixir_font_discovery/Cargo.toml
cargo fmt --manifest-path native/elixir_font_discovery/Cargo.toml -- --check
cargo clippy --manifest-path native/elixir_font_discovery/Cargo.toml --all-targets -- -D warnings
```

## Releasing

1. Update the version in `mix.exs`, the Rust crate, and `CHANGELOG.md`.
2. Commit and push the release.
3. Tag it as `vVERSION` and push the tag.
4. Wait for the precompiled-NIF workflow to attach every target to the GitHub
   release.
5. Generate the checksum manifest:

   ```sh
   mix rustler_precompiled.download ElixirFontDiscovery.Native --all --print
   ```

6. Commit `checksum-Elixir.ElixirFontDiscovery.Native.exs`.
7. Confirm the package contents with `mix hex.build --unpack`.
8. Publish to Hex with `mix hex.publish`.

## Licence

Elixir Font Discovery is released under the MIT License.
