# Supported platforms

- `jute-macos-aarch64`
- `jute-linux-x86_64`
- `jute-linux-aarch64`

Binaries are named `jute-<os>-<arch>`, taking `<os>` and `<arch>` verbatim from Rust's `std::env::consts::OS` and `std::env::consts::ARCH`.

Intel macOS is not supported.
