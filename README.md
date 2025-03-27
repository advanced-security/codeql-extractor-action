<!-- markdownlint-disable -->
<div align="center">
<h1>CodeQL Extractor Action</h1>

[![GitHub](https://img.shields.io/badge/github-%23121011.svg?style=for-the-badge&logo=github&logoColor=white)][github]
[![GitHub Actions](https://img.shields.io/github/actions/workflow/status/advanced-security/codeql-extractor-action/main.yml?style=for-the-badge)][github-actions]
[![GitHub Issues](https://img.shields.io/github/issues/advanced-security/codeql-extractor-action?style=for-the-badge)][github-issues]
[![GitHub Stars](https://img.shields.io/github/stars/advanced-security/codeql-extractor-action?style=for-the-badge)][github]
[![Licence](https://img.shields.io/github/license/Ileriayo/markdown-badges?style=for-the-badge)][license]

</div>
<!-- markdownlint-restore -->

## Overview

[CodeQL Extractor Action][github] is a GitHub Action that helps none-GitHub [CodeQL] Extractor to integrate with GitHub Actions.

## ✨ Features

- **Easy to use**: The action is designed to be simple and easy to integrate into your existing GitHub Actions workflows.
- **End-to-end workflow**: The action provides end-to-end workflow for extracting code from your repository and running CodeQL analysis.
- **Customizable**: The action allows you to customize the extraction process to fit your specific needs.

## Usage

```yml
- name: "CodeQL Extractor Action"
  uses: advanced-security/codeql-extractor-action@v0.1.0
  with:
    # Repository reference (e.g. "owner/repo", "owner/repo@ref")
    extractor: "advanced-security/codeql-extractor-iac"
    # [optional] Attest the authenticity of the extractor
    attestation: true
```

> !WARNING
> This action downloads the extractor from the GitHub repository. Make sure to use a trusted repository, owner, and extractor.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

<!-- Resoucres -->

[license]: ./LICENSE
[github]: https://github.com/advanced-security/codeql-extractor-action
[github-issues]: https://github.com/advanced-security/codeql-extractor-action/issues
[github-actions]: https://github.com/advanced-security/codeql-extractor-action/actions

[CodeQL]: https://codeql.github.com/
