# CI/CD spec

## Releasing a new version of jute

### Release pipeline spec

- Trigger a github action on pushes to `vx.y.z` git tag (should match `CARGO_PKG_VERSION` - assert that this is the case)
- Run a matrix over the platforms listed in [docs/specs/supported_platforms.md](./supported_platforms.md):
  - Apple silicon macOS
  - Intel Linux
  - ARM Linux
- Each runner should build the release binary (see [release build settings](#release-build-settings)) and then run `./target/<target>/release/jute test` with it, so that the binary we ship is the one that ran the tests
- Linux builds should use statically linked musl for max compatibility
- If testing and building on all runners succeeds, then create a github release with those builds as artifacts
- Binaries should use the same naming convention as in [docs/specs/supported_platforms.md](./supported_platforms.md)
- The github release should include a table of the file size of each binary (+ the total)

### Release build settings

jute is meant to be committed into a repo, and `jute self.install` writes a binary
for every supported platform into `.jute/bin/`, so a consuming repo carries the
total of all of them. Release builds are therefore built for size.

The toolchain is pinned in [rust-toolchain.toml](../../rust-toolchain.toml).
Release binaries rebuild the standard library from source:

```sh
RUSTC_BOOTSTRAP=1 \
RUSTFLAGS="-Zunstable-options -Cpanic=immediate-abort" \
cargo build --release --target <target> \
  -Z build-std=std,panic_abort \
  -Z build-std-features=optimize_for_size
```

This is what takes a musl binary from ~488kb to ~137kb. Roughly 110kb of the
original is backtrace symbolisation code (a DWARF parser, a zlib inflater and a
symbol demangler) reached from std's default panic hook. Because `strip = true`
those backtraces could never resolve to anything readable anyway, but the code is
genuinely reachable, so no post-link tool can remove it — only rebuilding std
without it can. `-Cpanic=immediate-abort` removes the remaining panic formatting
machinery, and accounts for over half the saving on its own.

`RUSTC_BOOTSTRAP=1` enables these unstable flags on the pinned stable toolchain,
so no nightly install is needed. The flags themselves are still unstable and can
change, but that now only lands when we choose to bump the pin.

The `install` task in `.jute/tasks.jute` uses the same settings so that a locally
built jute matches a released one — it is what `jute self.install` copies into
`.jute/bin/` for the current platform. It needs two extra flags,
`-Z target-applies-to-host --config target-applies-to-host=false`, because it has
no `--target` to pass: without one, cargo applies `RUSTFLAGS` to build scripts
too, and rebuilding std pulls in build scripts (from `libc`, `compiler_builtins`
and `std` itself) that then fail to link against the prebuilt host `core`.
Passing `--target` is enough to avoid this in CI, even on macOS where the target
triple and the host triple are the same.

Two things follow from this and are easy to trip over:

- A panic in a release binary aborts with no message, location or backtrace.
  jute reports errors through `Result` instead, so a panic means a bug — but it
  will be a bug with no diagnostic beyond `SIGABRT`.
- These settings must stay scoped to the release build step. `jute test` shells
  out to `cargo fmt`/`clippy`/`test`; if `RUSTFLAGS` leaked into those, the
  dev-profile test harness would inherit `immediate-abort` and could no longer
  report test failures, since it relies on unwinding.

### Download link url format

```
https://github.com/sgrowe/jute/releases/download/v{CARGO_PKG_VERSION}/jute-<os>-<arch>
```

## CI testing

On new pushes to any branch a github action should run the same test command over the same matrix as the release flow above. It should also build for each platform and output the binary file sizes in the same way. It should NOT create a new github release or otherwise publish anything however.

## Guidelines

- Set `RUST_BACKTRACE=full` to enable verbose error logging to help with diagnosing CI failures. Note this only affects the test suite, which is built with the dev profile; the release binary running it is built with `-Cpanic=immediate-abort` and produces no backtrace of its own (see [release build settings](#release-build-settings))
