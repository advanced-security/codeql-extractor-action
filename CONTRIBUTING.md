# Contributing

Hi there! We're thrilled that you'd like to contribute to this project. Your help is essential for keeping it great.

Contributions to this project are [released](https://help.github.com/articles/github-terms-of-service/#6-contributions-under-repository-license) to the public under the [project's open source license](LICENSE.md).

Please note that this project is released with a [Contributor Code of Conduct][code-of-conduct]. By participating in this project you agree to abide by its terms.

## Reporting Bugs

The best way to report a bug is to open an issue on GitHub. Please include as much information as possible, including:

- A clear description of the problem
- Steps to reproduce the problem
- The expected behavior
- The actual behavior

This will help us understand the issue and fix it more quickly.

## Suggesting Enhancements

If you have an idea for a new feature or enhancement, please open an issue on GitHub.

## Submitting Changes

1. [Fork][fork] and clone the repository
2. Create a new branch for your changes
3. Make your changes
4. Write tests for your changes (if applicable)
5. Run the tests to make sure everything is working

### Required Tools

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- [CodeQL](https://codeql.github.com/docs/codeql-cli/getting-started/)
  - `gh-codeql` is a great tool to help you with CodeQL CLI.

### Running Tests

To run the tests, use the following command:

```bash
cargo test
```

### Running Linter

To run the linter, use the following command:

```bash
cargo clippy
```

## Resources

- [How to Contribute to Open Source](https://opensource.guide/how-to-contribute/)
- [Using Pull Requests](https://help.github.com/articles/about-pull-requests/)
- [GitHub Help](https://help.github.com)

[fork]: https://github.com/advanced-security/codeql-extractor-action/fork
[pr]: https://github.com/advanced-security/codeql-extractor-action/compare
[code-of-conduct]: https://github.com/advanced-security/codeql-extractor-action

