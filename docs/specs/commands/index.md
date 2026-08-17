# `jute <command>`s

## Namespaces

Jute commands can be namespaced by having multiple `.jute/<namespace>.jute` files, they can then be run by executing `jute <namespace>.<cmd>`, for example `jute backend.build` runs the `build` task defined in `.jute/backend.jute`. Only one level of namespacing is supported.

## Default namespace

Tasks in `.jute/default.jute` do not have a namespace prefix. `jute dev` runs the `dev` task defined in `.jute/default.jute`

## Built in commands

Built in jute commands live in the `self.*` namespace, other than the special `jute help` command. Having a `.jute/self.jute` file is an error.

Built in commands:

- [`jute self.install`](./install_command.md)
