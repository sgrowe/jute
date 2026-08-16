# `--install` command

## Flow

- create `.jute/` dir if not exists
- create/overwrite `.jute/run` script from default
  - make it executable
- create default `.jute/tasks.jute` if not exists already (don't overwrite a users existing tasks files if they already have one)

- create `.jute/bin/` dir if not exists
- get the current OS and arch of the current binary using Rust's `std::env::consts::OS` etc, and copy `std::env::current_exe()` there
- TODO (need to setup CI first): for the remaining architectures download the binaries from GitHub releases using `curl`

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

## Default tasks.jute

Use `examples/default.jute`

## Questions

- How to handle distributing the cross platform binaries? Downloaded on demand from GH releases?
- How to handle the `run` script and how many platforms can it support? Probs macOS + linux is fine, but what about windows??
