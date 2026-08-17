# `jute self.install` command

## Flow

- create `.jute/` dir if not exists (in current working dir)
- create/overwrite `.jute/run` script (source: `src/templates/run`)
  - make it executable
- create default `.jute/tasks.jute` if it doesn’t already exist (don't overwrite a users existing tasks file if they already have one)
- create `.jute/bin/` dir if not exists
- get the current OS and arch of the current binary using Rust's `std::env::consts::OS` etc, and copy `std::env::current_exe()` there
- for the remaining platforms in [docs/specs/supported_platforms.md](./supported_platforms.md) download the binaries from GitHub releases using `curl` (see [docs/specs/ci_cd_flow.md](./ci_cd_flow.md) for the download URL format) and mark them as executable
  - fetch the binaries for the same version as the current executable (i.e. `CARGO_PKG_VERSION`)
  - fail on HTTP errors (e.g. curl's `--fail` flag)
  - obey HTTP redirects (e.g. the `--location` flag)
  - use 5 retries
  - max time per retry attempt: 5 mins
- for failed curl commands output the full command output (stdout and stderr) to the user (plus a "curl failed with exit code X") message and the `jute --install` command as a whole should fail

## Target folder structure

```
.jute/
  tasks.jute    <- where you define all your tasks and scripts
  run           <- executable script to run jute, so that in CI and other environments you can
                   immediately run `.jute/run <command>` without any installation needed

  bin/          <- binaries for each platform, `.jute/run` automatically executes the correct one
    jute-macos-aarch64
    jute-linux-x86_64
    jute-linux-aarch64
```

See also: [docs/specs/supported_platforms.md](./supported_platforms.md)

## Default tasks.jute

Use `examples/default.jute`
