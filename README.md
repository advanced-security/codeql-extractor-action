<!-- markdownlint-disable -->
<div align="center">
<h1>CodeQL Extractor Action</h1>

[![GitHub](https://img.shields.io/badge/github-%23121011.svg?style=for-the-badge&logo=github&logoColor=white)][github]
[![GitHub Actions](https://img.shields.io/github/actions/workflow/status/advanced-security/codeql-extractor-action/build.yml?style=for-the-badge)][github-actions]
[![GitHub Issues](https://img.shields.io/github/issues/advanced-security/codeql-extractor-action?style=for-the-badge)][github-issues]
[![GitHub Stars](https://img.shields.io/github/stars/advanced-security/codeql-extractor-action?style=for-the-badge)][github]
[![Licence](https://img.shields.io/github/license/Ileriayo/markdown-badges?style=for-the-badge)][license]

</div>
<!-- markdownlint-restore -->

## Overview

[CodeQL Extractor Action][github] is a GitHub Action that allows you to specify a CodeQL extractor to be used in your workflows as an author of an Extractor.
This action is designed to be used in conjunction with the [CodeQL][CodeQL] analysis tool, which is a powerful static analysis tool that can be used to find vulnerabilities in your code.

> [!WARNING]
> This action downloads the extractor from the GitHub repository. Make sure to use a trusted repository, owner, and extractor.

## ✨ Features

- **Easy to use**: The action is designed to be simple and easy to integrate into your existing GitHub Actions workflows.
- **End-to-end workflow**: The action provides end-to-end workflow for extracting code from your repository and running CodeQL analysis.
- **Customizable**: The action allows you to customize the extraction process to fit your specific needs.

## Usage

```yml
- name: "CodeQL Extractor Action"
  uses: advanced-security/codeql-extractor-action@v0.0.11
  with:
    # Repository reference (e.g. "owner/repo", "owner/repo@ref")
    extractor: "advanced-security/codeql-extractor-iac"
    # [optional]: Language(s) used to verify the extractor
    languages: "iac"
    # [optional] Attest the authenticity of the extractor
    attestation: true
```

## Q&A

**Whats is an Extractor?**

A CodeQL extractor is a tool that extracts code from a repository and prepares it for analysis by the CodeQL engine. It is used to convert the code into a format that can be analyzed by CodeQL.

**How do I create an Extractor?**

To create an extractor, you need to create a GitHub repository that contains the extractor releases as an artifact / assest in a GitHub release.
The extractor should be a Tarball file that contains the compiled extractor and all other necessary files for the extractor to run.


## Maintainers 

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="10%"><a href="https://geekmasher.dev"><img src="https://avatars.githubusercontent.com/u/2772944?v=3?s=100" width="100px;" alt="Mathew Payne"/><br /><sub><b>Mathew Payne</b></sub></a><br /><a href="https://github.com/advanced-security/codeql-extractor-iac/commits?author=geekmasher" title="Code">💻</a> <a href="#research-geekmasher" title="Research">🔬</a> <a href="#maintenance-geekmasher" title="Maintenance">🚧</a> <a href="#security-geekmasher" title="Security">🛡️</a> <a href="#ideas-geekmasher" title="Ideas, Planning, & Feedback">🤔</a></td>
    </tr>
  </tbody>
</table>
<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->
<!-- ALL-CONTRIBUTORS-LIST:END -->

## Support

Please create [GitHub Issues][github-issues] if there are bugs or feature requests.

This project uses [Sematic Versioning (v2)](https://semver.org/) and with major releases, breaking changes will occur.

## License

This project is licensed under the terms of the MIT open source license.
Please refer to [MIT][license] for the full terms.


<!-- Resoucres -->

[license]: ./LICENSE
[github]: https://github.com/advanced-security/codeql-extractor-action
[github-issues]: https://github.com/advanced-security/codeql-extractor-action/issues
[github-actions]: https://github.com/advanced-security/codeql-extractor-action/actions
[github-discussions]: https://github.com/advanced-security/codeql-extractor-action/discussions

[CodeQL]: https://codeql.github.com/
