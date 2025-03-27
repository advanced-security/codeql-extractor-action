use std::path::PathBuf;

use anyhow::{Context, Result};
use ghactions::{ActionTrait, ToolCache, group, groupend};
use ghastoolkit::CodeQL;
use log::{debug, info};

mod action;
mod extractors;

use action::Action;

#[tokio::main]
async fn main() -> Result<()> {
    let action = Action::init()?;
    debug!("Action :: {:?}", action);

    group!("Setting up Extractor");

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

    let extractor = extractors::fetch_extractor(
        &client,
        &extractor_repo,
        action.attestation(),
        &extractor_path,
    )
    .await
    .context("Failed to fetch extractor")?;
    log::info!("Extractor :: {:?}", extractor);

    let codeql = CodeQL::init()
        .search_path(extractor)
        .build()
        .await
        .context("Failed to create CodeQL instance")?;
    log::info!("CodeQL :: {:?}", codeql);

    let languages = codeql.get_languages().await?;
    log::info!("Languages :: {:?}", languages);

    // TODO: This is erroring during development
    // action.set_extractor_path(extractor_path.display().to_string());

    groupend!();

    group!("Running extractor");

    groupend!();

    Ok(())
}
