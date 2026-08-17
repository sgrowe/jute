# jute

## Problem

We want a nicer way of defining our project's tasks and scripts than a `Makefile`, but if we use a task runner (like [just](https://github.com/casey/just)) then we can't reuse our scripts and tasks on CI or other machines without installing that task runner first. That's annoying to setup and maintain, and slows down every CI run.

`jute` tries to bypass this tradeoff by being lightweight enough to be stored directly in your repo. You’re cloning your repo anyway and the `jute` binary is only ~0.13mb so the performance cost is minimal and there’s no extra CI steps to maintain.

When installed globally (for convenience) you can run `jute <command>`, and when embedded in your repo you can run `.jute/run <command>`, whichever works best for the situation at hand.

## Example `tasks.jute`

```tasks.jute

simple_example:
  echo "hello world"

complex_example:
  in crates/server:
    echo "hello from the ./crates/server/ folder"
  in crates/frontend:
    npm install
    npm build
  cargo build

```

## Installed folder structure

Run `jute self.install` in the root of your repo to install it. jute will create the following folder structure:

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
