//! CodeQL installation and management utilities
//!
//! This module provides helper functions for downloading and installing CodeQL,
//! particularly through alternative methods like GitHub CLI when the standard
//! installation process fails.

use anyhow::{Context, Result};
use ghactions::ActionTrait;
use ghastoolkit::CodeQL;

use crate::action::Action;

/// Download and install the CodeQL CLI, with fallback to GitHub CLI if necessary
pub async fn codeql_download(action: &Action) -> Result<CodeQL> {
    let token = action.get_token();

    let mut codeql = CodeQL::init()
        .build()
        .await
        .context("Failed to create CodeQL instance")?;
    log::debug!("CodeQL :: {codeql:?}");

    if !codeql.is_installed().await {
        let codeql_version = action.codeql_version();
        log::info!("CodeQL not installed, installing `{codeql_version}`...");

        // Try to install with authentication first (if token is available)
        if !token.is_empty() {
            let octocrab_auth = action.octocrab_with_token(token)?;
            if let Ok(_) = codeql.install(&octocrab_auth, codeql_version).await {
                log::info!("CodeQL installed using authentication");
                return Ok(codeql);
            } else {
                log::warn!(
                    "Failed to install CodeQL with authentication, trying without authentication..."
                );
            }
        }

        // Try to install without authentication
        let octocrab = action.octocrab_without_token()?;
        if let Ok(_) = codeql.install(&octocrab, codeql_version).await {
            log::info!("CodeQL installed without authentication");
            return Ok(codeql);
        } else {
            log::warn!("Failed to install CodeQL without authentication");
            log::info!("Attempting to install CodeQL using GitHub CLI...");
        }

        let location = gh_codeql_download(codeql_version)
            .await
            .context("Failed to download CodeQL using GitHub CLI")?;
        // Reinitialize CodeQL with the new path
        codeql = CodeQL::init()
            .path(location)
            .build()
            .await
            .context("Failed to create CodeQL instance after GitHub CLI installation")?;

        log::info!("CodeQL installed");
    } else {
        log::info!("CodeQL already installed");
    }

    Ok(codeql)
}

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
async fn gh_codeql_download(codeql_version: &str) -> Result<String> {
    log::info!("Downloading CodeQL Extension for GitHub CLI...");
    log::debug!("Running command: gh extensions install github/gh-codeql");
    let status = tokio::process::Command::new("gh")
        .args(&["extensions", "install", "github/gh-codeql"])
        .env(
            "GH_TOKEN",
            std::env::var("GITHUB_TOKEN").unwrap_or_default(),
        )
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
        .env(
            "GH_TOKEN",
            std::env::var("GITHUB_TOKEN").unwrap_or_default(),
        )
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
        .env(
            "GH_TOKEN",
            std::env::var("GITHUB_TOKEN").unwrap_or_default(),
        )
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
