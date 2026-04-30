# Installation

Prebuilt binaries ship for macOS, Linux, and Windows on every tagged release. Pick whichever channel fits your environment.

## via npm

```bash
npm install -g @anekoique/ark
```

The npm package wraps the prebuilt binary; no Rust toolchain needed.

## via Cargo

```bash
cargo install --git https://github.com/Anekoique/ark ark-cli --locked
```

Requires a working Rust toolchain. The crate's `rust-toolchain.toml` pins nightly (used for `rustfmt` unstable options); `cargo install` reads it automatically when invoked from a clone, but installs from the registry compile cleanly on stable as well.

## via prebuilt binary

Download the archive for your platform from the [GitHub Releases page](https://github.com/Anekoique/ark/releases) and put `ark` somewhere on your `PATH`.

## Verifying the install

```bash
ark --version
```

Expected output: `ark <version>` matching your install. If `ark` isn't found, check that the install path (npm's global bin, Cargo's `~/.cargo/bin`, or wherever you put the prebuilt binary) is on your shell's `PATH`.

## Updating

- **npm**: `npm update -g @anekoique/ark`
- **cargo**: `cargo install --git https://github.com/Anekoique/ark ark-cli --locked --force`
- **prebuilt**: download the new archive and replace the binary.

After a CLI update, run `ark upgrade` inside each Ark-managed project to refresh the embedded templates. See [`ark upgrade`](../reference/ark-upgrade.md) for the conflict-resolution policy.
