#![allow(dead_code)]
//! Action module for defining and handling the GitHub Action's inputs, outputs, and core functionality
//!
//! This module contains the Action struct which represents the GitHub Action and implements
//! the necessary functionality to process inputs, validate configurations, and manage outputs.
use std::path::PathBuf;

use anyhow::{Context, Result};
use ghactions::prelude::*;
use ghactions_core::repository::reference::RepositoryReference as Repository;
use ghastoolkit::{CodeQL, CodeQLPack, codeql::CodeQLLanguage};

/// ASCII art banner for the CodeQL Extractor Action
pub const BANNER: &str = r#"   ___          _        ____  __    __      _     _        _   
  / __\___   __| | ___  /___ \/ /   /__\_  _| |_  /_\   ___| |_ 
 / /  / _ \ / _` |/ _ \//  / / /   /_\ \ \/ / __|//_\\ / __| __|
/ /__| (_) | (_| |  __/ \_/ / /___//__  >  <| |_/  _  \ (__| |_ 
\____/\___/ \__,_|\___\___,_\____/\__/ /_/\_\\__\_/ \_/\___|\__|"#;

/// Version of the CodeQL Extractor Action, pulled from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Authors of the CodeQL Extractor Action, pulled from Cargo.toml
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

/// This action is for 3rd party CodeQL extractors to be used in GitHub Actions
#[derive(Actions, Debug, Clone, Default)]
#[action(
    // Name of the Action
    name = "CodeQL Extractor Action",
    // Description of the Action
    description = "This action is for 3rd party CodeQL extractors to be used in GitHub Actions",
    // Path to the action.yml file
    path = "./action.yml",
    // Path to the Dockerfile
    image = "./action.Dockerfile",

    icon = "shield",
    color = "blue",

)]
pub struct Action {
    /// GitHub Token
    #[input(description = "GitHub Token", default = "${{ github.token }}")]
    token: String,

    /// GitHub Repository where the extractor is located
    #[input(
        description = "GitHub Repository where the extractor(s) is located",
        split = ",",
        required = true
    )]
    extractors: Vec<String>,

    /// Language(d) to use
    #[input(description = "Language(s) to use", split = ",", required = true)]
    languages: Vec<String>,

    /// Queries packs to use
    #[input(description = "Query Pack(s) to use", split = ",")]
    packs: Vec<String>,

    /// Allow empty database. This allows for an extractor to error out if no database was
    /// created dur to no source code being found for that language.
    #[input(
        description = "Allow empty database",
        default = false,
        rename = "allow-empty-database"
    )]
    allow_empty_database: bool,

    /// CodeQL Version
    #[input(
        description = "CodeQL Version",
        rename = "codeql-version",
        default = "latest"
    )]
    codeql_version: String,

    /// Working Directory (defualt: `./`)
    #[input(
        description = "Working Directory",
        rename = "working-directory",
        default = "./"
    )]
    working_directory: String,

    /// Re-write SARIF file tool name
    #[input(
        description = "Re-write SARIF file tool name",
        rename = "sarif-tool-name",
        default = "true"
    )]
    sarif_tool_name: bool,

    /// Attestation
    #[input(description = "Attestation", default = "false")]
    attestation: bool,

    /// SARIF Results Directory
    #[output(description = "SARIF Results Directory", rename = "sarif-results")]
    sarif_results: String,

    /// Version of the extractor to use
    #[output(description = "Version of the extractor to use")]
    version: String,

    /// Path to the extractor
    #[output(description = "Path to the extractor", rename = "extractor-path")]
    extractor_path: String,
}

impl Action {
    /// Returns the GitHub Token for the action
    pub fn get_token(&self) -> String {
        if !self.token.is_empty() {
            log::debug!("Using provided token");
            self.token.clone()
        } else if let Ok(gh_token) = std::env::var("GITHUB_TOKEN") {
            log::debug!("No token provided, using GITHUB_TOKEN environment variable");
            gh_token
        } else {
            log::debug!("No token provided, and GITHUB_TOKEN environment variable not set");
            String::new()
        }
    }

    /// Returns the working directory for the action
    ///
    /// If no working directory is provided, the current directory is used.
    /// Otherwise, the provided directory is resolved to an absolute path.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The resolved working directory path
    pub fn working_directory(&self) -> Result<PathBuf> {
        if self.working_directory.is_empty() {
            log::debug!("No working directory provided, using the current directory");
            return std::env::current_dir().context("Failed to get current directory");
        }
        log::debug!("Using the provided working directory");
        std::path::PathBuf::from(&self.working_directory)
            .canonicalize()
            .context(format!(
                "Failed to get working directory `{}`",
                self.working_directory
            ))
    }

    /// Gets the repository references for the extractors
    ///
    /// If no extractor repositories are provided, the current repository is used.
    /// Otherwise, the provided repositories are parsed into Repository objects.
    ///
    /// # Returns
    /// - `Result<Vec<Repository>>`: A list of parsed repository references
    pub fn extractor_repository(&self) -> Result<Vec<Repository>> {
        if self.extractors.is_empty() {
            log::debug!("No extractor repository provided, using the current repository");
            return Ok(vec![Repository::parse(&self.get_repository()?)?]);
        }

        log::debug!(
            "Using the provided extractor repositories: {:?}",
            self.extractors
        );

        let repos: Vec<Repository> = self
            .extractors
            .iter()
            .filter_map(|ext| match Repository::parse(ext) {
                Ok(repo) => {
                    log::debug!(
                        "Successfully parsed repository: {} / {}",
                        repo.owner,
                        repo.name
                    );
                    Some(repo)
                }
                Err(e) => {
                    log::warn!("Failed to parse extractor repository `{}`: {}", ext, e);
                    None
                }
            })
            .collect();

        log::debug!("Parsed {} repositories", repos.len());
        Ok(repos)
    }

    /// Returns the list of languages to use for CodeQL analysis.
    pub fn languages(&self) -> Vec<CodeQLLanguage> {
        log::debug!("Getting languages for analysis: {:?}", self.languages);
        let languages = self
            .languages
            .iter()
            .map(|lang| CodeQLLanguage::from(lang.as_str()))
            .collect();
        log::debug!("Converted to CodeQL languages: {:?}", languages);
        languages
    }

    /// Gets the possible directories for CodeQL operations.
    ///
    /// This function identifies potential locations for CodeQL operation directories in the following order:
    /// 1. The `.codeql` directory in the GitHub workspace (if running in GitHub Actions)
    /// 2. The `.codeql` directory in the current working directory
    /// 3. The `.codeql` directory in the GitHub Actions runner's temp directory (if available)
    /// 4. The `.codeql` directory in the system's temporary directory
    ///
    /// Each path is checked for existence and created if necessary by the caller.
    ///
    /// # Returns
    /// - `Result<Vec<PathBuf>>`: A vector of possible directory paths for CodeQL operations
    ///
    /// # Errors
    /// - If `working_directory()` fails
    /// - If path canonicalization fails
    fn get_codeql_directories(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Local CodeQL directory in the working directory
        if let Ok(working_dir) = self.working_directory() {
            if let Ok(local_codeql) = working_dir.join(".codeql").canonicalize() {
                log::debug!("Local working directory found: {}", local_codeql.display());
                paths.push(local_codeql);
            }
        }

        // GITHUB_WORKSPACE
        if let Ok(github_workspace) = std::env::var("GITHUB_WORKSPACE") {
            log::debug!("GITHUB_WORKSPACE found: {}", github_workspace);
            paths.push(PathBuf::from(github_workspace).join(".codeql"));
        }

        // Runner temp directory
        if let Ok(runner_temp) = std::env::var("RUNNER_TEMP") {
            log::debug!("RUNNER_TEMP found: {}", runner_temp);
            paths.push(PathBuf::from(runner_temp).join(".codeql"));
        }
        // temp_dir
        if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
            log::debug!("System temp directory found: {}", temp_dir.display());
            paths.push(temp_dir.join(".codeql"));
        }

        paths
    }

    /// Returns the directory to use for CodeQL operations.
    ///
    /// Gets the CodeQL directory to use for the action. It will first check if a local
    /// `.codeql` directory exists in the working directory parent. If not, it will
    /// use the `RUNNER_TEMP` directory. If neither exists, it will create a new
    /// `.codeql` directory in the working directory parent.
    ///
    /// It uses the parent of the working directory to to stop issues where the
    /// database/sarif files gets indexed by CodeQL.
    pub fn get_codeql_dir(&self) -> Result<PathBuf> {
        let paths = self.get_codeql_directories();
        if paths.is_empty() {
            return Err(anyhow::anyhow!("No valid CodeQL directories were found"));
        }
        log::debug!("Possible CodeQL directories: {:?}", paths);

        for path in paths {
            if !path.exists() {
                log::debug!("Creating CodeQL directory at `{}`", path.display());
                if std::fs::create_dir_all(&path).is_ok() {
                    return Ok(path);
                } else {
                    log::warn!("Failed to create CodeQL directory at `{}`", path.display());
                }
            } else {
                log::debug!("CodeQL directory already exists at `{}`", path.display());
                return Ok(path);
            }
        }

        Err(anyhow::anyhow!("Failed to create CodeQL directory",))
    }

    /// Validates the provided languages against the supported CodeQL languages.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the provided languages are not supported.
    pub fn validate_languages(&self, codeql_languages: &Vec<CodeQLLanguage>) -> Result<()> {
        for lang in self.languages() {
            let mut supported = false;
            log::debug!("Validating language `{lang}`");
            for codeql_lang in codeql_languages {
                if lang.language().to_lowercase() == codeql_lang.language().to_lowercase() {
                    log::debug!("Language `{lang}` is supported");
                    supported = true;
                    break;
                }
            }
            if !supported {
                return Err(anyhow::anyhow!(
                    "Language(s) `{}` not supported by the extractor",
                    self.languages()
                        .iter()
                        .map(|lang| lang.language())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        Ok(())
    }

    /// Returns the CodeQL version to use.
    ///
    /// If the CodeQL version is not provided, it defaults to "latest".
    pub fn codeql_version(&self) -> &str {
        if self.codeql_version.is_empty() {
            log::debug!("No CodeQL version provided, using the latest version");
            return "latest";
        }
        &self.codeql_version
    }

    /// Installs the specified CodeQL packs.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the packs cannot be installed.
    pub async fn install_packs(&self, codeql: &CodeQL) -> Result<()> {
        log::info!("Installing CodeQL Packs");
        for pack in &self.packs {
            log::debug!("Installing pack `{pack}`");

            let qlpack = CodeQLPack::try_from(pack.clone())?;

            if !qlpack.is_installed().await {
                log::info!(
                    "QLPack `{}` is not installed, installing it now",
                    qlpack.full_name()
                );
                if pack.starts_with("./") {
                    qlpack
                        .install(codeql)
                        .await
                        .context(format!("Failed to install pack `{pack}`"))?;
                } else {
                    codeql
                        .pack(&qlpack)
                        .download()
                        .await
                        .context(format!("Failed to download pack `{pack}`"))?;
                }
            } else {
                log::info!("QLPack `{}` is already installed", qlpack.full_name());
            }
            log::info!("QLPack :: {qlpack:#?}");
        }
        Ok(())
    }

    /// Returns whether attestation is enabled.
    pub fn attestation(&self) -> bool {
        log::debug!("Attestation enabled: {}", self.attestation);
        self.attestation
    }

    /// Returns whether empty databases are allowed.
    pub fn allow_empty_database(&self) -> bool {
        log::debug!("Allow empty database: {}", self.allow_empty_database);
        self.allow_empty_database
    }

    pub fn sarif_tool_name(&self) -> bool {
        log::debug!("Re-write SARIF tool name: {}", self.sarif_tool_name);
        self.sarif_tool_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to create a test Action instance with predefined values
    ///
    /// Creates an Action with:
    /// - A single extractor repository "owner/repo"
    /// - A single language "iac"
    /// - Default values for all other fields
    fn action() -> Action {
        Action {
            extractors: vec!["owner/repo".to_string()],
            languages: vec!["iac".to_string()],
            ..Default::default()
        }
    }

    /// Test that language validation works correctly
    ///
    /// Tests two scenarios:
    /// 1. When a language is specified that isn't supported by CodeQL (should error)
    /// 2. When a language is specified that is supported by CodeQL (should pass)
    #[test]
    fn test_validate_languages() {
        let action = action();
        let codeqllanguages = vec![CodeQLLanguage::from("Python")];

        let result = action.validate_languages(&codeqllanguages);
        assert!(result.is_err(), "Expected error for unsupported language");

        let codeqllanguages = vec![
            CodeQLLanguage::from("Python"),
            CodeQLLanguage::from("java"),
            CodeQLLanguage::from("iac"),
        ];
        let result = action.validate_languages(&codeqllanguages);
        assert!(result.is_ok(), "Expected no error for supported language");
    }
}
