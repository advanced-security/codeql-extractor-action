#![allow(dead_code)]
use anyhow::Result;
use ghactions::prelude::*;
use ghactions_core::repository::reference::RepositoryReference as Repository;
use ghastoolkit::codeql::CodeQLLanguage;

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
    #[input(description = "GitHub Token")]
    token: String,

    /// GitHub Repository where the extractor is located
    #[input(
        description = "GitHub Repository where the extractor is located",
        required = true
    )]
    extractor: String,

    /// Language(d) to use
    #[input(description = "Language(s) to use", split = ",", required = true)]
    language: Vec<String>,

    /// Attestation
    #[input(description = "Attestation", default = "false")]
    attestation: bool,

    /// Version of the extractor to use
    #[output(description = "Version of the extractor to use")]
    version: String,
    /// Path to the extractor
    #[output(description = "Path to the extractor")]
    extractor_path: String,
}

impl Action {
    /// Gets the repository to use for the extractor. If the repository is not provided,
    /// it will use the repository that the action is running in.
    pub fn extractor_repository(&self) -> Result<Repository> {
        let repo = if self.extractor.is_empty() {
            log::debug!("No extractor repository provided, using the current repository");
            self.get_repository()?
        } else {
            log::debug!("Using the provided extractor repository");
            self.extractor.clone()
        };
        log::info!("Extractor Repository :: {}", repo);

        Ok(Repository::parse(&repo)?)
    }

    pub fn languages(&self) -> Vec<CodeQLLanguage> {
        self.language
            .iter()
            .map(|lang| CodeQLLanguage::from(lang.as_str()))
            .collect()
    }

    pub fn validate_languages(&self, codeql_languages: &Vec<CodeQLLanguage>) -> Result<()> {
        for lang in self.languages() {
            let mut supported = false;
            log::debug!("Validating language `{}`", lang);
            for codeql_lang in codeql_languages {
                if lang.language().to_lowercase() == codeql_lang.language().to_lowercase() {
                    log::debug!("Language `{}` is supported", lang);
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

    pub fn attestation(&self) -> bool {
        self.attestation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action() -> Action {
        Action {
            extractor: "owner/repo".to_string(),
            language: vec!["iac".to_string()],
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
