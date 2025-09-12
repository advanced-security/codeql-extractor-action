use anyhow::{Context, Result};
use ghastoolkit::CodeQL;

/// Download the CodeQL CLI using the GitHub CLI
pub async fn gh_codeql_download(codeql_version: &str) -> Result<String> {
    log::info!("Downloading CodeQL Extension for GitHub CLI...");
    tokio::process::Command::new("gh")
        .args(&["extensions", "install", "github/gh-codeql"])
        .status()
        .await
        .context("Failed to execute `gh extensions install github/gh-codeql` command")?;

    log::info!("Setting CodeQL version to {codeql_version}...");
    tokio::process::Command::new("gh")
        .args(&["codeql", "set-version", codeql_version])
        .status()
        .await
        .context("Failed to execute `gh codeql set-version` command")?;

    log::info!("Install CodeQL stub...");
    tokio::process::Command::new("gh")
        .args(&["codeql", "install-stub"])
        .status()
        .await
        .context("Failed to execute `gh codeql install-stub` command")?;

    let codeql = CodeQL::new().await;
    if codeql.is_installed().await {
        log::info!("CodeQL CLI installed successfully via GitHub CLI");
    } else {
        log::error!("CodeQL CLI installation via GitHub CLI failed");
        return Err(anyhow::anyhow!("CodeQL CLI installation failed"));
    }

    Ok("/usr/local/bin/codeql".to_string())
}
