#![allow(dead_code)]
use anyhow::Result;
use ghactions::prelude::*;
use ghastoolkit::Repository;

/// This action is for 3rd party CodeQL extractors to be used in GitHub Actions
#[derive(Actions, Debug, Clone)]
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

    pub fn languages(&self) -> Vec<String> {
        self.language.clone()
    }

    pub fn attestation(&self) -> bool {
        self.attestation
    }
}
