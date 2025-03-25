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
    #[input(description = "GitHub Repository where the extractor is located")]
    extractor_repository: String,
}

impl Action {
    /// Gets the repository to use for the extractor. If the repository is not provided,
    /// it will use the repository that the action is running in.
    pub fn extractor_repository(&self) -> Result<Repository> {
        let repo = if self.extractor_repository.is_empty() {
            log::debug!("No extractor repository provided, using the current repository");
            self.get_repository()?
        } else {
            log::debug!("Using the provided extractor repository");
            self.extractor_repository.clone()
        };
        log::info!("Extractor Repository :: {}", repo);

        Ok(Repository::parse(&repo)?)
    }
}
