# Releasing a new version of jute

## Release pipeline spec

- Trigger a github action on pushes to `vx.y.z` git tag (should match `CARGO_PKG_VERSION` - assert that this is the case)
- Run a matrix over:
  - Apple silicon mac (which also cross compiles to intel mac)
  - Intel linux
  - ARM linux
- Each runner should run `cargo run --release -- test` to ensure jute works on that platform
- Linux builds should use statically linked musl for max compatibility
- If testing and building on all runners succeeds, then create a github release with those builds as artifacts
- Binaries should use the same naming convention as in [docs/specs/install_command.md]
- The github release should include a table of the file size of each binary (+ the total)

## Download link url format

```
https://github.com/sgrowe/jute/releases/download/v{CARGO_PKG_VERSION}/jute-<os>-<arch>
```
