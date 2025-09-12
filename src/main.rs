//! Main module for the CodeQL Extractor Action
//!
//! This module contains the main function and orchestrates the entire workflow
//! for setting up CodeQL, fetching and configuring extractors, and running analyses.
use anyhow::{Context, Result};
use ghactions::{ActionTrait, group, groupend};
use ghactions_core::RepositoryReference;
use ghastoolkit::codeql::database::queries::CodeQLQueries;
use ghastoolkit::prelude::*;
use log::{debug, info};

mod action;
mod codeql;
mod extractors;

use crate::codeql::codeql_download;
use action::{AUTHORS, Action, BANNER, VERSION};

/// Main function that drives the CodeQL Extractor Action workflow
///
/// This function:
/// 1. Initializes the Action
/// 2. Sets up CodeQL
/// 3. Fetches and configures the extractors
/// 4. Creates databases for each language
/// 5. Runs analyses on the databases
/// 6. Processes and updates the SARIF results
#[tokio::main]
async fn main() -> Result<()> {
    let mut action = Action::init()?;

    info!("{BANNER} - v{VERSION} by {AUTHORS}\n");

    debug!("Action :: {action:?}");

    // Use a client without a token to not use Action' token which will fail
    // as the token won't have access to the CodeQL repository.
    let octocrab = action.octocrab_without_token()?;

    let cwd = action
        .working_directory()
        .context("Failed to get working directory")?;
    log::info!("Working Directory :: {cwd:?}");
    let codeql_dir = action
        .get_codeql_dir()
        .context("Failed to get CodeQL directory")?;
    log::info!("CodeQL Directory :: {codeql_dir:?}");

    let databases = codeql_dir.join("databases");
    let sarif_output = codeql_dir.join("results");

    group!("Setting up CodeQL");

    let mut codeql = codeql_download(&action)
        .await
        .context("Failed to set up CodeQL")?;
    log::info!(
        "CodeQL CLI Version :: {}",
        codeql.version().unwrap_or_default()
    );

    // Packs installation
    action.install_packs(&codeql).await?;

    groupend!();
    group!("Setting up Extractor");

    // Extractor
    let extractor_repos = action
        .extractor_repository()
        .context("Failed to get extractor repository")?;

    let extractor_path = codeql_dir.join("extractors");
    if !extractor_path.exists() {
        std::fs::create_dir_all(&extractor_path)
            .with_context(|| format!("Failed to create directory {extractor_path:?}"))?;
        info!("Created Extractor Directory :: {extractor_path:?}");
    }

    log::debug!(
        "Creating extractors container for {} repositories",
        extractor_repos.len()
    );
    let mut extractors: Vec<(CodeQLExtractor, RepositoryReference)> = Vec::new();

    for extractor_repo in extractor_repos.iter() {
        log::info!(
            "Fetching extractor from repository: {} / {}",
            extractor_repo.owner,
            extractor_repo.name
        );
        log::debug!("Repository reference details: {:?}", extractor_repo);

        let extractor_path = match extractors::fetch_extractor(
            &octocrab,
            extractor_repo,
            action.attestation(),
            &extractor_path,
        )
        .await
        {
            Ok(path) => {
                log::debug!("Successfully fetched extractor to {}", path.display());
                path
            }
            Err(e) => {
                log::error!(
                    "Failed to fetch extractor from {}/{}: {}",
                    extractor_repo.owner,
                    extractor_repo.name,
                    e
                );
                return Err(e).context("Failed to fetch extractor");
            }
        };
        log::info!("Extractor :: {extractor_path:?}");

        log::debug!(
            "Appending search path to CodeQL instance: {}",
            extractor_path.display()
        );
        codeql.append_search_path(&extractor_path);

        log::info!("Loading CodeQL extractor from path: {extractor_path:?}");
        let extractor = match CodeQLExtractor::load_path(extractor_path.clone()) {
            Ok(ext) => {
                log::debug!(
                    "Successfully loaded extractor: name={}, version={}, languages={:?}",
                    ext.name,
                    ext.version,
                    ext.languages()
                );
                ext
            }
            Err(e) => {
                log::error!(
                    "Failed to load extractor from {}: {}",
                    extractor_path.display(),
                    e
                );
                return Err(anyhow::anyhow!("Failed to load extractor: {}", e));
            }
        };

        log::debug!("Adding extractor to collection");
        extractors.push((extractor, extractor_repo.clone()));
    }

    let languages = codeql
        .get_languages()
        .await
        .context("Failed to get languages")?;
    log::info!("Languages :: {languages:#?}");

    if !action.languages().is_empty() {
        log::info!("Validating language(s) :: {:?}", action.languages());

        action
            .validate_languages(&languages)
            .context("Failed to validate languages")?;
        log::info!("Language(s) validated");
    } else {
        log::info!("No languages provided, using all available languages");
    }

    log::info!("CodeQL :: {codeql:#?}");

    std::fs::create_dir_all(&sarif_output).context("Failed to create results directory")?;

    groupend!();

    for (extractor, reporef) in extractors {
        // The language is the name of the extractor
        let language = extractor.name.to_string();

        group!(format!("Running {} extractor", language));

        log::info!("Running extractor for language :: {language}");

        let database_path = databases.join(format!("db-{language}"));
        log::info!("Database Path :: {database_path:?}");
        if database_path.exists() {
            std::fs::remove_dir_all(&database_path).with_context(|| {
                format!("Failed to remove database directory {database_path:?}")
            })?;
        }

        let sarif_path = sarif_output.join(format!("{language}-results.sarif"));

        let mut database = CodeQLDatabase::init()
            .name(action.get_repository_name()?)
            .source(cwd.display().to_string())
            .path(database_path.display().to_string())
            .language(language.to_string())
            .build()
            .context("Failed to create database")?;

        log::info!("Creating CodeQL database for language: {}", language);
        log::debug!(
            "Database creation parameters for: {}",
            database_path.display()
        );

        let start_time = std::time::Instant::now();
        match codeql.database(&database).overwrite().create().await {
            Ok(_) => {
                let elapsed = start_time.elapsed();
                log::debug!("Successfully created database :: {database:?}");
                log::info!(
                    "Database creation completed in {:.2} seconds",
                    elapsed.as_secs_f64()
                );
            }
            Err(e) => {
                log::error!("Failed to create database: {e:?}");
                log::debug!("Database creation error details: {:?}", e);

                if action.allow_empty_database() {
                    log::warn!(
                        "Empty database allowed by configuration, continuing with next language"
                    );
                    continue;
                } else {
                    log::error!("Empty database not allowed, aborting");
                    return Err(anyhow::anyhow!("Failed to create database: {e:?}"));
                }
            }
        }

        // TODO: Queries
        let queries = CodeQLQueries::parse(format!("{}/{language}-queries", reporef.owner.clone()))
            .context("Failed to parse queries")?;
        log::info!("Queries :: {queries:?}");

        groupend!();

        group!(format!("Running {language} analysis"));

        log::info!("Starting CodeQL analysis for language: {}", language);
        log::debug!(
            "Analysis configuration: database={}, queries={:?}, output={}",
            database_path.display(),
            queries,
            sarif_path.display()
        );

        let analysis_start_time = std::time::Instant::now();
        match codeql
            .database(&database)
            .queries(queries)
            .sarif(sarif_path.clone())
            .analyze()
            .await
        {
            Ok(_) => {
                let elapsed = analysis_start_time.elapsed();
                log::info!("Analysis complete in {:.2} seconds", elapsed.as_secs_f64());
                log::debug!("Successfully analyzed database and generated SARIF output");
            }
            Err(ghastoolkit::GHASError::SerdeError(e)) => {
                log::warn!("Failed to parse SARIF: {e:?}");
                log::debug!("SARIF parsing error details: {:?}", e);
            }
            Err(e) => {
                log::error!("Failed to analyze database: {e:?}");
                log::debug!("Analysis error details: {:?}", e);
            }
        }

        log::info!("Post-processing SARIF results");

        extractors::update_sarif(&sarif_path, extractor.display_name.clone())
            .context("Failed to update SARIF file with extractor information")?;

        // Reload the database to get analysis info
        database.reload()?;
        log::info!("CodeQL Database LoC :: {}", database.lines_of_code());

        log::info!("SARIF Output Path :: {sarif_path:?}");

        log::info!("Analysis complete :: {database:?}");
        groupend!();
    }

    // If the action is running in Actions, the SARIF file must be a relative path
    // This is because we assume that this code is running in a container which mounts
    // the repository at /github/workspace
    if let Ok(_) = std::env::var("CI") {
        // If running in a CI environment, set the SARIF as a relative path
        let relative_path = sarif_output.strip_prefix(&cwd).unwrap_or(&sarif_output);
        log::debug!(
            "CI environment detected, setting SARIF path as relative: {}",
            relative_path.display()
        );
        action.set_sarif_results(relative_path.display().to_string());
    } else {
        log::debug!("Setting SARIF path as absolute: {}", sarif_output.display());
        action.set_sarif_results(sarif_output.display().to_string());
    }

    log::info!("All databases created and analyzed");

    Ok(())
}
