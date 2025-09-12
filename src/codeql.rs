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
    log::debug!("Running command: gh extensions install github/gh-codeql");
    let status = tokio::process::Command::new("gh")
        .args(&["extensions", "install", "github/gh-codeql"])
        .status()
        .await
        .context("Failed to execute `gh extensions install github/gh-codeql` command")?;

    if !status.success() {
        log::error!(
            "Failed to install GitHub CLI CodeQL extension. Exit code: {:?}",
            status.code()
        );
        return Err(anyhow::anyhow!(
            "GitHub CLI CodeQL extension installation failed with exit code: {:?}",
            status.code()
        ));
    }
    log::debug!("GitHub CLI CodeQL extension installed successfully");

    log::info!("Setting CodeQL version to {codeql_version}...");
    log::debug!("Running command: gh codeql set-version {codeql_version}");
    let status = tokio::process::Command::new("gh")
        .args(&["codeql", "set-version", codeql_version])
        .status()
        .await
        .context("Failed to execute `gh codeql set-version` command")?;

    if !status.success() {
        log::error!(
            "Failed to set CodeQL version. Exit code: {:?}",
            status.code()
        );
        return Err(anyhow::anyhow!(
            "Setting CodeQL version failed with exit code: {:?}",
            status.code()
        ));
    }
    log::debug!("CodeQL version set to {codeql_version} successfully");

    log::info!("Installing CodeQL stub...");
    log::debug!("Running command: gh codeql install-stub");
    let status = tokio::process::Command::new("gh")
        .args(&["codeql", "install-stub"])
        .status()
        .await
        .context("Failed to execute `gh codeql install-stub` command")?;

    if !status.success() {
        log::error!(
            "Failed to install CodeQL stub. Exit code: {:?}",
            status.code()
        );
        return Err(anyhow::anyhow!(
            "CodeQL stub installation failed with exit code: {:?}",
            status.code()
        ));
    }
    log::debug!("CodeQL stub installed successfully");

    let codeql = CodeQL::new().await;
    if codeql.is_installed().await {
        log::info!("CodeQL CLI installed successfully via GitHub CLI");
    } else {
        log::error!("CodeQL CLI installation via GitHub CLI failed");
        return Err(anyhow::anyhow!("CodeQL CLI installation failed"));
    }

    Ok("/usr/local/bin/codeql".to_string())
}
