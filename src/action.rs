#![allow(dead_code)]
use std::path::PathBuf;

use anyhow::{Context, Result};
use ghactions::prelude::*;
use ghactions_core::repository::reference::RepositoryReference as Repository;
use ghastoolkit::{CodeQL, CodeQLPack, codeql::CodeQLLanguage};

pub const BANNER: &str = r#"   ___          _        ____  __    __      _     _        _   
  / __\___   __| | ___  /___ \/ /   /__\_  _| |_  /_\   ___| |_ 
 / /  / _ \ / _` |/ _ \//  / / /   /_\ \ \/ / __|//_\\ / __| __|
/ /__| (_) | (_| |  __/ \_/ / /___//__  >  <| |_/  _  \ (__| |_ 
\____/\___/ \__,_|\___\___,_\____/\__/ /_/\_\\__\_/ \_/\___|\__|"#;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
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

    /// Gets the repository to use for the extractor. If the repository is not provided,
    /// it will use the repository that the action is running in.
    pub fn extractor_repository(&self) -> Result<Vec<Repository>> {
        if self.extractors.is_empty() {
            log::debug!("No extractor repository provided, using the current repository");
            return Ok(vec![Repository::parse(&self.get_repository()?)?]);
        }

        log::debug!("Using the provided extractor repository");

        Ok(self
            .extractors
            .iter()
            .filter_map(|ext| {
                Repository::parse(ext)
                    .context(format!("Failed to parse extractor repository `{ext}`"))
                    .ok()
            })
            .collect::<Vec<Repository>>())
    }

    pub fn languages(&self) -> Vec<CodeQLLanguage> {
        self.languages
            .iter()
            .map(|lang| CodeQLLanguage::from(lang.as_str()))
            .collect()
    }

    pub fn get_codeql_dir(&self) -> Result<PathBuf> {
        let paths = vec![
            // Local CodeQL directory in the working directory
            self.working_directory()?.join(".codeql"),
            // Runner temp directory
            PathBuf::from(std::env::var("RUNNER_TEMP").unwrap_or_else(|_| "/tmp".to_string()))
                .join(".codeql"),
        ];

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

    pub fn codeql_version(&self) -> &str {
        if self.codeql_version.is_empty() {
            log::debug!("No CodeQL version provided, using the latest version");
            return "latest";
        }
        &self.codeql_version
    }

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

    pub fn attestation(&self) -> bool {
        self.attestation
    }

    pub fn allow_empty_database(&self) -> bool {
        self.allow_empty_database
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action() -> Action {
        Action {
            extractors: vec!["owner/repo".to_string()],
            languages: vec!["iac".to_string()],
            ..Default::default()
        }
    }

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
