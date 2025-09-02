---
applyTo: '**/*.rs'
---

This is a GitHub Action that that allows you to specify a CodeQL extractor to be used in your workflows as an author of an Extractor.
It is designed to be used in conjunction with the [CodeQL][CodeQL] analysis tool, which is a powerful static analysis tool that can be used to find vulnerabilities in your code.

The project is written in Rust and used the [ghactions](https://crates.io/crates/ghactions) crate to simplify the development of GitHub Actions in Rust.
The action is built using a Debian based Docker image.

## Guildelins

- Use cargo fmt to format the code.
- Use cargo clippy to lint the code.
- Always write documentation for public functions and modules.
- Write unit tests for all public functions.
- Use `log::info!`, `log::warn!`, `log::error!` for logging.

## Testing

You can test the Rust code locally using `cargo tests`.

```sh
cargo test
```

This will run all the tests in the project and display the results in the terminal.
Validate the output of the tests to ensure that everything is working as expected.
If the tests fail, debug the code and fix any issues before proceeding.
