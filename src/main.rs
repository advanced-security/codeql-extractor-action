use anyhow::{Context, Result};
use ghactions::{ActionTrait, group, groupend};
use ghactions_core::RepositoryReference;
use ghastoolkit::prelude::*;
use ghastoolkit::{codeql::database::queries::CodeQLQueries};
use log::{debug, info};

mod action;
mod extractors;

use action::{AUTHORS, Action, BANNER, VERSION};

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
    let codeql_dir = action.get_codeql_dir()?;

    let databases = codeql_dir.join("databases");
    let sarif_output = codeql_dir.join("results");

    group!("Setting up CodeQL");

    let mut codeql = CodeQL::init()
        .build()
        .await
        .context("Failed to create CodeQL instance")?;
    log::debug!("CodeQL :: {codeql:?}");

    if !codeql.is_installed().await {
        let codeql_version = action.codeql_version();
        log::info!("CodeQL not installed, installing `{codeql_version}`...");
        codeql
            .install(&octocrab, codeql_version)
            .await
            .context("Failed to install CodeQL")?;
        log::info!("CodeQL installed");
    } else {
        log::info!("CodeQL already installed");
    }
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

    let mut extractors: Vec<(CodeQLExtractor, RepositoryReference)> = Vec::new();

    for extractor_repo in extractor_repos.iter() {
        log::info!(
            "Fetching extractor from repository: {} / {}",
            extractor_repo.owner,
            extractor_repo.name
        );

        let extractor_path = extractors::fetch_extractor(
            &octocrab,
            extractor_repo,
            action.attestation(),
            &extractor_path,
        )
        .await
        .context("Failed to fetch extractor")?;
        log::info!("Extractor :: {extractor_path:?}");

        codeql.append_search_path(&extractor_path);

        log::info!("Extractor Path :: {extractor_path:?}");
        let extractor =
            CodeQLExtractor::load_path(extractor_path).context("Failed to load extractor")?;

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

    groupend!();

    std::fs::create_dir_all(&sarif_output).context("Failed to create results directory")?;

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

        log::info!("Creating database...");
        match codeql.database(&database).overwrite().create().await {
            Ok(_) => {
                log::debug!("Created database :: {database:?}");
            }
            Err(e) => {
                log::error!("Failed to create database: {e:?}");
                if action.allow_empty_database() {
                    log::warn!("Allowing empty database");
                    continue;
                } else {
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

        match codeql
            .database(&database)
            .queries(queries)
            .output(sarif_path.clone())
            .analyze()
            .await
        {
            Ok(_) => {
                log::info!("Analysis complete");
            }
            Err(ghastoolkit::GHASError::SerdeError(e)) => {
                log::warn!("Failed to parse SARIF: {e:?}");
            }
            Err(e) => {
                log::error!("Failed to analyze database: {e:?}");
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

    action.set_sarif_results(sarif_output.display().to_string());

    log::info!("All databases created and analyzed");

    Ok(())
}
