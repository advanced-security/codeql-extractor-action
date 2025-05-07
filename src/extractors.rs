use std::path::PathBuf;

use anyhow::Result;
use ghactions_core::repository::reference::RepositoryReference as Repository;
use octocrab::models::repos::{Asset, Release};

async fn fetch_releases(client: &octocrab::Octocrab, repository: &Repository) -> Result<Release> {
    let release = if let Some(rel) = &repository.reference {
        client
            .repos(repository.owner.clone(), repository.name.clone())
            .releases()
            .get_by_tag(&rel)
            .await?
    } else {
        // Get Latest Release
        client
            .repos(repository.owner.clone(), repository.name.clone())
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
    attest: bool,
    output: &PathBuf,
) -> Result<PathBuf> {
    let extractor_tarball = output.join("extractor.tar.gz");
    let extractor_pack = output.join("extractor-pack");
    let extractor_path = extractor_pack.join("codeql-extractor.yml");

    let toolcache = ghactions::ToolCache::new();

    if !extractor_tarball.exists() {
        log::info!("Downloading asset to {:?}", extractor_tarball);

        let release = fetch_releases(client, repository).await?;

        let Some(release_asset) = release.assets.iter().find(|a| a.name.ends_with(".tar.gz"))
        else {
            return Err(anyhow::anyhow!("No asset found"));
        };
        log::info!("Asset URL :: {}", release_asset.browser_download_url);

        let asset: Asset = client.get(release_asset.url.clone(), None::<&()>).await?;

        toolcache.download_asset(&asset, &extractor_tarball).await?;
    }

    if attest {
        log::info!("Attesting asset {:?}", extractor_tarball);

        let output = tokio::process::Command::new("gh")
            .arg("attestation")
            .arg("verify")
            .arg("--owner")
            .arg(repository.owner.clone())
            .arg(&extractor_tarball)
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Attestation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        log::info!("Attestation successful");
    } else {
        log::info!("No attestation requested");
    }

    if !extractor_pack.exists() {
        log::info!("Extracting asset to {:?}", extractor_path);
        toolcache
            .extract_archive(&extractor_tarball, &output)
            .await?;

        if !extractor_path.exists() {
            return Err(anyhow::anyhow!("Extractor not found"));
        }
    }

    Ok(extractor_pack.canonicalize()?)
}
