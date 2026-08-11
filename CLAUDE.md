# jute

A task runner. `.jute/tasks.jute` files are read by `tokeniser.rs`, turned into
the `ast.rs` types by `parser.rs`, and executed from `main.rs`.

## After making changes

Always run

```sh
cargo run -- validate
```

This bootstraps the project and then uses it to run the tests, formatter and clippy (see `.jute/tasks.jute`)

Also, after making larger changes, get an adversarial code review to check for bugs, and whether the code can be simplified.

## Tests

- In tests use the public API of the code under test as much as possible. Avoid additions or changes to the public API purely for the sake of the tests
- Always prefer deep equality assertions (such as `assert_eq!(xyz, vec![...])`) over vaguer, weaker assertions such as `.find(...).is_some()` - even if it means larger assertions.

## Comments

- Avoid comments that simply restate what is obvious from the function name or the code itself. Only add a comment for info which is non-obvious. Comments that remain should be concise and straightforward.
