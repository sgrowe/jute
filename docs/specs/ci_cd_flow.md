# CI/CD spec

## Releasing a new version of jute

### Release pipeline spec

- Trigger a github action on pushes to `vx.y.z` git tag (should match `CARGO_PKG_VERSION` - assert that this is the case)
- Run a matrix over:
  - Apple silicon mac
  - Intel linux
  - ARM linux
- Each runner should run `cargo run --release -- test` to ensure jute works on that platform
- Linux builds should use statically linked musl for max compatibility
- If testing and building on all runners succeeds, then create a github release with those builds as artifacts
- Binaries should use the same naming convention as in [docs/specs/install_command.md]
- The github release should include a table of the file size of each binary (+ the total)

### Download link url format

```
https://github.com/sgrowe/jute/releases/download/v{CARGO_PKG_VERSION}/jute-<os>-<arch>
```

## CI testing

On new pushes to any branch a github action should run the same test command over the same matrix as the release flow above. It should also build for each platform and output the binary file sizes in the same way. It should NOT create a new github release or otherwise publish anything however.

## Guidelines

- Set `RUST_BACKTRACE=full` to enable verbose error logging to help with diagnosing CI failures
