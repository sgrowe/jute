# jute

A task runner. `.jute/tasks.jute` files are read by `tokeniser.rs`, turned into
the `ast.rs` types by `parser.rs`, and executed from `main.rs`.

## After making changes

Always run all three, in this order:

```sh
cargo fmt
cargo test
cargo clippy --fix --all-targets --allow-dirty
```

## Tests

- In tests use the public API of the code under test as much as possible. Avoid additions or changes to the public API purely for the sake of the tests
- Always prefer deep equality assertions (such as `assert_eq!(xyz, vec![...])`) over vaguer, weaker assertions such as `.find(...).is_some()` - even if it means larger assertions.
