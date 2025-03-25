use std::path::PathBuf;

use anyhow::Result;
use ghastoolkit::Repository;
use ghastoolkit::codeql::CodeQLExtractor;
use octocrab::models::repos::{Asset, Release};

async fn fetch_releases(client: &octocrab::Octocrab, repository: &Repository) -> Result<Release> {
    let release = if let Some(rel) = repository.reference() {
        client
            .repos(repository.owner(), repository.name())
            .releases()
            .get_by_tag(rel)
            .await?
    } else {
        // Get Latest Release
        client
            .repos(repository.owner(), repository.name())
            .releases()
            .get_latest()
            .await?
    };

    log::debug!("Release :: {} - {:?}", release.tag_name, release.created_at);

    Ok(release)
}

/// Fetch the CodeQL Extractor from the repository
///
/// Finds the correct asset based on ending in `.tar.gz`.
pub async fn fetch_extractor(
    client: &octocrab::Octocrab,
    repository: &Repository,
    output: &PathBuf,
) -> Result<CodeQLExtractor> {
    let release = fetch_releases(client, repository).await?;

    let Some(release_asset) = release.assets.iter().find(|a| a.name.ends_with(".tar.gz")) else {
        return Err(anyhow::anyhow!("No asset found"));
    };
    log::info!("Asset URL :: {}", release_asset.browser_download_url);

    let asset: Asset = client.get(release_asset.url.clone(), None::<&()>).await?;

    let extractor_tarball = output.join("extractor.tar.gz");
    let extractor_path = output.join("extractor-pack").join("codeql-extractor.yml");

    let toolcache = ghactions::ToolCache::new();

    if !extractor_tarball.exists() {
        log::info!("Downloading asset to {:?}", extractor_tarball);

        toolcache.download_asset(&asset, &extractor_tarball).await?;
    }

    if extractor_path.exists() {
        log::info!("Removing existing asset {:?}", extractor_path);
        std::fs::remove_dir_all(&extractor_path)?;
    }

    log::info!("Extracting asset to {:?}", extractor_path);
    toolcache
        .extract_archive(&extractor_tarball, &output)
        .await?;

    if !extractor_path.exists() {
        return Err(anyhow::anyhow!("Extractor not found"));
    }

    log::info!("Loading CodeQL Extractor from {:?}", extractor_path);
    let extractor = CodeQLExtractor::load_path(extractor_path)?;

    Ok(extractor)
}
