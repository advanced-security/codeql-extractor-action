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

    /// GitHub Repositories where the extractors are located
    #[input(
        description = "GitHub Repositories where the extractors are located",
        required = true,
        split = ","
    )]
    extractor: Vec<String>,

    /// Language(d) to use, e.g. `iac`, `javascript`, `python`, etc.
    #[input(description = "Language(s) to use", split = ",")]
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
    /// Gets the repositories to use for the extractors. If no repositories are provided,
    /// it will use the repository that the action is running in.
    pub fn extractor_repositories(&self) -> Result<Vec<Repository>> {
        let mut repositories = Vec::new();
        for extractor in &self.extractor {
            let repo = if extractor.is_empty() {
                log::debug!("No extractor repository provided, using the current repository");
                self.get_repository()?
            } else {
                log::debug!("Using the provided extractor repository");
                extractor.clone()
            };
            log::info!("Extractor Repository :: {}", repo);
            repositories.push(Repository::parse(&repo)?);
        }
        Ok(repositories)
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
            extractor: vec!["owner/repo1".to_string(), "owner/repo2".to_string()],
            language: vec!["iac".to_string()],
            ..Default::default()
        }
    }

    fn action_with_extractors(extractors: Vec<String>) -> Action {
        Action {
            extractor: extractors,
            language: vec!["iac".to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn test_extractor_repositories() {
        let action = action();
        let repositories = action.extractor_repositories().unwrap();
        assert_eq!(repositories.len(), 2);
        assert_eq!(repositories[0].to_string(), "owner/repo1");
        assert_eq!(repositories[1].to_string(), "owner/repo2");
    }

    #[test]
    fn test_extractor_repositories_multiple() {
        let action =
            action_with_extractors(vec!["owner/repo1".to_string(), "owner/repo2".to_string()]);
        let repositories = action.extractor_repositories().unwrap();
        assert_eq!(repositories.len(), 2);
        assert_eq!(repositories[0].to_string(), "owner/repo1");
        assert_eq!(repositories[1].to_string(), "owner/repo2");
    }

    #[test]
    fn test_extractor_repositories_single() {
        let action = action_with_extractors(vec!["owner/repo1".to_string()]);
        let repositories = action.extractor_repositories().unwrap();
        assert_eq!(repositories.len(), 1);
        assert_eq!(repositories[0].to_string(), "owner/repo1");
    }

    #[test]
    fn test_extractor_repositories_empty() {
        let action = action_with_extractors(vec![]);
        let result = action.extractor_repositories();
        assert!(result.is_err(), "Expected error for empty extractor list");
    }

    #[test]
    fn test_extractor_repositories_invalid_format() {
        let action = action_with_extractors(vec!["invalid_repo_format".to_string()]);
        let result = action.extractor_repositories();
        assert!(
            result.is_err(),
            "Expected error for invalid repository format"
        );
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
