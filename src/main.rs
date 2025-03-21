use anyhow::Result;
use ghactions::{group, groupend, ActionTrait};
use log::info;
use octocrab::{models::issues::Issue, params::State};
use std::error::Error;

mod action;

use action::Action;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let action = Action::init()?;

    info!("Action :: {:?}", action);

    group!("Main Workflow");

    info!("Repository: `{}`", action.get_repository()?);

    let client = action.octocrab()?;

    // https://docs.rs/octocrab/latest/octocrab/index.html
    // Example to get all the active issues
    let issues_pages = client
        .issues(
            action.get_repository_owner()?,
            action.get_repository_name()?,
        )
        .list()
        .state(State::Open)
        .per_page(50)
        .send()
        .await?;

    for issue in client.all_pages::<Issue>(issues_pages).await? {
        info!(" >> {} -> {}", issue.id, issue.title);
    }

    groupend!();

    Ok(())
}
