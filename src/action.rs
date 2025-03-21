use ghactions::prelude::*;

#[derive(Actions, Debug, Clone)]
#[action(
    // Name of the Action
    name = "CodeQL Extractor Action",
    // Description of the Action
    description = "This action is for 3rd party CodeQL extractors to be used in GitHub Actions",
    // Path to the action.yml file
    path = "./action.yml",
    // Path to the Dockerfile
    image = "./container/action.Dockerfile",

    icon = "shield",
    color = "blue",

)]
pub struct Action {
    /// GitHub Token
    #[input(description = "GitHub Token")]
    token: String,

    /// GitHub Repository
    #[input(description = "GitHub Repository")]
    repository: String,
}
