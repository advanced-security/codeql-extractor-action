//! CodeQL installation and management utilities
//!
//! This module provides helper functions for downloading and installing CodeQL,
//! particularly through alternative methods like GitHub CLI when the standard 
//! installation process fails.

use anyhow::{Context, Result};
use ghastoolkit::CodeQL;

/// Download and install the CodeQL CLI using the GitHub CLI
///
/// This function serves as a fallback installation method when the standard CodeQL 
/// installation process fails. It uses the GitHub CLI to:
/// 1. Install the gh-codeql extension
/// 2. Set the specified CodeQL version
/// 3. Install the CodeQL stub for command-line access
///
/// # Arguments
/// * `codeql_version` - The version of CodeQL to download (e.g., "latest" or a specific version)
///
/// # Returns
/// * `Result<String>` - Path to the installed CodeQL binary or an error
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
