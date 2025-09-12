//! CodeQL Extractor Fetcher
use anyhow::{Context, Result};
use ghactions_core::repository::reference::RepositoryReference as Repository;
use octocrab::models::repos::{Asset, Release};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};

/// Fetches a release from a GitHub repository
///
/// If the repository reference includes a specific tag, it fetches that release.
/// Otherwise, it fetches the latest release.
///
/// # Arguments
/// * `client` - The Octocrab client to use for API requests
/// * `repository` - The repository reference containing owner, name, and optional tag
///
/// # Returns
/// * `Result<Release>` - The fetched release or an error
async fn fetch_releases(client: &octocrab::Octocrab, repository: &Repository) -> Result<Release> {
    log::debug!(
        "Fetching releases for repository: {}/{}",
        repository.owner,
        repository.name
    );
    let release = if let Some(rel) = &repository.reference {
        log::info!("Fetching release by tag: {}", rel);
        log::debug!(
            "API call: repos/{}/{}/releases/tags/{}",
            repository.owner,
            repository.name,
            rel
        );
        match client
            .repos(repository.owner.clone(), repository.name.clone())
            .releases()
            .get_by_tag(&rel)
            .await
        {
            Ok(release) => release,
            Err(e) => {
                log::error!("Failed to fetch release by tag '{}': {}", rel, e);
                return Err(anyhow::anyhow!(
                    "Failed to fetch release by tag '{}': {}",
                    rel,
                    e
                ));
            }
        }
    } else {
        log::info!("Fetching latest release");
        log::debug!(
            "API call: repos/{}/{}/releases/latest",
            repository.owner,
            repository.name
        );
        // Get Latest Release
        match client
            .repos(repository.owner.clone(), repository.name.clone())
            .releases()
            .get_latest()
            .await
        {
            Ok(release) => release,
            Err(e) => {
                log::error!("Failed to fetch latest release: {}", e);
                return Err(anyhow::anyhow!("Failed to fetch latest release: {}", e));
            }
        }
    };

    log::info!("Release :: {} - {:?}", release.tag_name, release.created_at);

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
    let extractor_zip = output.join(format!("{}.zip", &repository.name));

    log::debug!("Extractor Tarball :: {extractor_tarball:?}");
    let extractor_pack = output.join(&repository.name);

    log::info!("Extractor Path :: {extractor_pack:?}");

    let toolcache = ghactions::ToolCache::new();

    let extractor_archive = if !extractor_tarball.exists() && !extractor_zip.exists() {
        log::info!("Downloading asset to {extractor_tarball:?}");

        let release = fetch_releases(client, repository).await?;

        let (release_asset, file_format) = match release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".tar.gz") || a.name.ends_with(".zip"))
        {
            Some(asset) if asset.name.ends_with(".tar.gz") => (asset, "tar"),
            Some(asset) if asset.name.ends_with(".zip") => (asset, "zip"),
            _ => {
                return Err(anyhow::anyhow!("No suitable asset found for extractor"));
            }
        };
        log::info!("Asset URL :: {}", release_asset.browser_download_url);

        let asset: Asset = client.get(release_asset.url.clone(), None::<&()>).await?;

        let extractor_archive = if file_format == "tar" {
            extractor_tarball.clone()
        } else {
            extractor_zip.clone()
        };

        toolcache
            .download_asset(&asset, &extractor_archive)
            .await
            .context(format!("Extractor Archive: {extractor_tarball:?}"))
            .context("Failed to download extractor")?;
        extractor_archive
    } else {
        if extractor_tarball.exists() {
            extractor_tarball.clone()
        } else {
            extractor_zip.clone()
        }
    };

    // Get and log the size of the extractor archive
    if let Ok(metadata) = std::fs::metadata(&extractor_archive) {
        let size_bytes = metadata.len();
        let size_mb = size_bytes as f64 / 1_048_576.0; // Convert to MB (1 MB = 1,048,576 bytes)
        log::info!(
            "Extractor archive size: {:.2} MB ({} bytes)",
            size_mb,
            size_bytes
        );
    } else {
        log::warn!("Unable to get size information for the extractor archive");
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

    log::debug!("Extractor Archive :: {extractor_archive:?}");

    if !extractor_pack.exists() {
        log::info!("Extracting asset to {extractor_pack:?}");

        toolcache
            .extract_archive(&extractor_archive, &extractor_pack)
            .await
            .context(format!("Extractor Archive: {extractor_tarball:?}"))
            .context("Failed to extract extractor")?;
    }

    // Find `codeql-extractor.yml` in the extracted directory using glob
    log::debug!("Searching for codeql-extractor.yml in {}", extractor_pack.display());
    if let Some(glob_result) = glob::glob(
        &extractor_pack
            .join("**/codeql-extractor.yml")
            .to_string_lossy(),
    )?.next() {
        match glob_result {
            Ok(path) => {
                // TODO: Load and check the extractor configuration
                log::debug!("Found extractor configuration at: {path:?}");
                let full_path = path.parent().unwrap().to_path_buf().canonicalize()?;
                log::debug!("Using extractor directory: {}", full_path.display());
                
                // Linux and Macos
                #[cfg(unix)]
                {
                    update_tools_permisisons(&full_path)?;
                }

                return Ok(full_path);
            }
            Err(e) => {
                log::error!("Failed to access extractor path: {e}");
                return Err(anyhow::anyhow!("Failed to access extractor path: {e}"));
            }
        }
    } else {
        log::warn!("No codeql-extractor.yml found in {}", extractor_pack.display());
    }
    Ok(extractor_pack)
}

/// Update the SARIF file with the extractor information (CodeQL ${language})
///
/// Updates only the `runs.0.tool.driver` section of the SARIF file to include
/// information about which extractor was used. This helps in distinguishing
/// results from different CodeQL extractors when analyzing multiple languages.
///
/// # Arguments
/// * `path` - Path to the SARIF file that needs to be updated
/// * `extractor` - Name of the extractor to be added to the SARIF metadata
///
/// # Returns
/// * `Result<()>` - Success or an error if the SARIF file couldn't be updated
pub fn update_sarif(path: &PathBuf, extractor: String) -> Result<()> {
    log::debug!(
        "Updating SARIF file at {} with extractor information: {}",
        path.display(),
        extractor
    );

    // Read SARIF file
    let sarif_content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            log::error!("Failed to read SARIF file {}: {}", path.display(), e);
            return Err(anyhow::anyhow!(
                "Failed to read SARIF file: {:?} - {}",
                path,
                e
            ));
        }
    };

    // Parse SARIF JSON
    let mut sarif_json: serde_json::Value = match serde_json::from_str(&sarif_content) {
        Ok(json) => json,
        Err(e) => {
            log::error!(
                "Failed to parse SARIF file {} as JSON: {}",
                path.display(),
                e
            );
            return Err(anyhow::anyhow!(
                "Failed to parse SARIF file: {:?} - {}",
                path,
                e
            ));
        }
    };

    log::debug!(
        "SARIF structure: has runs={}, has results={}",
        sarif_json.get("runs").is_some(),
        sarif_json
            .get("runs")
            .and_then(|r| r.get(0))
            .and_then(|r| r.get("results"))
            .is_some()
    );

    // Update the tool driver name
    if let Some(tool) = sarif_json
        .get_mut("runs")
        .and_then(|runs| runs.get_mut(0))
        .and_then(|run| run.get_mut("tool"))
    {
        if let Some(driver) = tool.get_mut("driver") {
            let new_name = format!("CodeQL - {}", extractor);
            log::debug!(
                "Updating tool.driver.name from '{}' to '{}'",
                driver
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown"),
                new_name
            );
            driver["name"] = serde_json::Value::String(new_name);
            log::info!("Updated SARIF file with extractor: {extractor}");
        } else {
            log::warn!("No 'driver' field found in SARIF file");
        }
    } else {
        log::warn!("No 'runs' or 'tool' field found in SARIF file");
    }

    // Serialize and write back to file
    let data = match serde_json::to_string(&sarif_json) {
        Ok(json) => json,
        Err(e) => {
            log::error!("Failed to serialize updated SARIF JSON: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to serialize SARIF JSON: {:?} - {}",
                path,
                e
            ));
        }
    };

    // Write the updated SARIF back to the file
    if let Err(e) = std::fs::write(path, &data) {
        log::error!("Failed to write updated SARIF file: {}", e);
        return Err(anyhow::anyhow!(
            "Failed to write SARIF file: {:?} - {}",
            path,
            e
        ));
    }

    log::debug!("Successfully updated SARIF file at {}", path.display());
    Ok(())
}

/// Update the permissions for tool scripts (*.sh) and the extractor executables
///
/// Makes shell scripts and extractor binaries executable by setting appropriate permissions.
/// Looks for tools in standard locations for Linux (linux64/extractor) and macOS (osx64/extractor).
///
/// # Arguments
/// * `path` - The base path where tools are located
///
/// # Returns
/// * `Result<()>` - Success or an error if permissions couldn't be set
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

/// Sets the file permissions to be executable (read and execute for all users)
///
/// Sets the permissions to 0o555 (r-xr-xr-x) which allows reading and
/// execution by all users, but no write permissions.
///
/// # Arguments
/// * `path` - The path to the file whose permissions should be set
///
/// # Returns
/// * `Result<()>` - Success or an error if permissions couldn't be set
fn set_permissions(path: &PathBuf) -> Result<()> {
    log::info!("Setting permissions for :: {:?}", path);

    // Get current permissions for logging
    if let Ok(metadata) = std::fs::metadata(path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            log::debug!("Current permissions: {:o}", metadata.permissions().mode());
        }
    } else {
        log::warn!("Could not get current file metadata for {}", path.display());
    }

    log::debug!("Setting permissions to 0o555 (r-xr-xr-x)");
    let perms = std::fs::Permissions::from_mode(0o555);

    match std::fs::set_permissions(&path, perms) {
        Ok(_) => {
            log::debug!("Successfully set permissions for {}", path.display());
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to set permissions for {}: {}", path.display(), e);
            Err(anyhow::anyhow!(
                "Failed to set permissions for {}: {}",
                path.display(),
                e
            ))
        }
    }
}
