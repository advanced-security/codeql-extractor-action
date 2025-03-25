use std::path::PathBuf;

use anyhow::{Context, Result};
use ghactions::{ActionTrait, ToolCache, group, groupend};
use log::{debug, info};

mod action;
mod extractors;

use action::Action;

#[tokio::main]
async fn main() -> Result<()> {
    let action = Action::init()?;
    debug!("Action :: {:?}", action);

    group!("Initialise Workflow");

    let client = octocrab::instance();

    let toolcache = ToolCache::new();
    debug!("ToolCache :: {:?}", toolcache);

    // Extractor
    let extractor_repo = action.extractor_repository()?;
    info!("Extractor Repository :: {}", extractor_repo);

    let extractor_path = PathBuf::from("./extractors");
    if !extractor_path.exists() {
        std::fs::create_dir(&extractor_path)
            .with_context(|| format!("Failed to create directory {:?}", extractor_path))?;
        info!("Created Extractor Directory :: {:?}", extractor_path);
    }

    let extractor = extractors::fetch_extractor(&client, &extractor_repo, &extractor_path).await?;
    log::info!("Extractor :: {:?}", extractor);

    groupend!();

    group!("Download and install extractor");

    // TODO: Validate the extractor

    groupend!();

    Ok(())
}
