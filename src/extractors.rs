use std::{os::unix::fs::PermissionsExt, path::PathBuf};

use anyhow::{Context, Result};
use ghactions_core::{repository::reference::RepositoryReference as Repository, toolcache::tool};
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
    let extractor_tarball = output.join(format!("{}.tar.gz", &repository.name));
    log::debug!("Extractor Tarball :: {extractor_tarball:?}");
    let extractor_pack = output.join(&repository.name);

    log::info!("Extractor Path :: {extractor_pack:?}");

    let toolcache = ghactions::ToolCache::new();

    if !extractor_tarball.exists() {
        log::info!("Downloading asset to {extractor_tarball:?}");

        let release = fetch_releases(client, repository).await?;

        let Some(release_asset) = release.assets.iter().find(|a| a.name.ends_with(".tar.gz"))
        else {
            return Err(anyhow::anyhow!("No asset found"));
        };
        log::info!("Asset URL :: {}", release_asset.browser_download_url);

        let asset: Asset = client.get(release_asset.url.clone(), None::<&()>).await?;

        toolcache
            .download_asset(&asset, &extractor_tarball)
            .await
            .context(format!("Extractor Archive: {extractor_tarball:?}"))
            .context("Failed to download extractor")?;
    }

    if attest {
        log::info!("Attesting asset {extractor_tarball:?}");

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
        log::info!("Extracting asset to {extractor_pack:?}");

        toolcache
            .extract_archive(&extractor_tarball, &extractor_pack)
            .await
            .context(format!("Extractor Archive: {extractor_tarball:?}"))
            .context("Failed to extract extractor")?;
    }

    // Find `codeql-extractor.yml` in the extracted directory using glob
    for glob in glob::glob(
        &extractor_pack
            .join("**/codeql-extractor.yml")
            .to_string_lossy(),
    )? {
        match glob {
            Ok(path) => {
                log::debug!("Extractor Path :: {path:?}");
                let full_path = path.parent().unwrap().to_path_buf().canonicalize()?;
                // Linux and Macos
                #[cfg(unix)]
                {
                    update_tools_permisisons(&full_path)?;
                }

                return Ok(full_path);
            }
            Err(e) => {
                log::error!("Failed to find extractor: {e}");
                return Err(anyhow::anyhow!("Failed to find extractor: {e}"));
            }
        }
    }
    Ok(extractor_pack)
}

/// Update the permissions for tool scripts (*.sh) and the extractor (extractor)
fn update_tools_permisisons(path: &PathBuf) -> Result<()> {
    let tools_path = path.join("tools");
    log::info!("Tools :: {tools_path:?}");

    if tools_path.exists() {
        log::debug!("Found tools directory at {tools_path:?}");

        // Linux
        let linux_extractor = tools_path.join("linux64").join("extractor");
        if linux_extractor.exists() {
            set_permissions(&linux_extractor)?;
        }
        // Macos
        let macos_extractor = tools_path.join("osx64").join("extractor");
        if macos_extractor.exists() {
            set_permissions(&macos_extractor)?;
        }

        for file in std::fs::read_dir(&tools_path)? {
            let file = file?;
            let path = file.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "sh") {
                log::debug!("Setting executable permissions for {path:?}");
                set_permissions(&path)?;
            }
        }
    }
    Ok(())
}

/// Sets the file permissions to be executable
fn set_permissions(path: &PathBuf) -> Result<()> {
    log::info!("Setting permissions for :: {:?}", path);
    let perms = std::fs::Permissions::from_mode(0o555);
    std::fs::set_permissions(&path, perms)?;
    Ok(())
}
