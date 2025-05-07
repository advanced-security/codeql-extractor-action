use std::path::PathBuf;

use anyhow::{Context, Result};
use ghactions::{ActionTrait, group, groupend};
use ghastoolkit::codeql::database::queries::CodeQLQueries;
use ghastoolkit::{CodeQL, CodeQLDatabase};
use log::{debug, info};

mod action;
mod extractors;

use action::Action;

#[tokio::main]
async fn main() -> Result<()> {
    let action = Action::init()?;
    debug!("Action :: {:?}", action);

    let client = action.octocrab()?;

    group!("Setting up CodeQL");

    let mut codeql = CodeQL::init()
        .build()
        .await
        .context("Failed to create CodeQL instance")?;

    if !codeql.is_installed().await {
        let codeql_version = action.codeql_version();
        log::info!("CodeQL not installed, installing `{}`...", codeql_version);
        codeql
            .install(&client, codeql_version)
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
    let extractor_repo = action
        .extractor_repository()
        .context("Failed to get extractor repository")?;

    let extractor_path = PathBuf::from("./extractors");
    if !extractor_path.exists() {
        std::fs::create_dir(&extractor_path)
            .with_context(|| format!("Failed to create directory {:?}", extractor_path))?;
        info!("Created Extractor Directory :: {:?}", extractor_path);
    }

    let extractor = extractors::fetch_extractor(
        &client,
        &extractor_repo,
        action.attestation(),
        &extractor_path,
    )
    .await
    .context("Failed to fetch extractor")?;
    log::info!("Extractor :: {:?}", extractor);

    codeql.append_search_path(extractor);

    let languages = codeql
        .get_languages()
        .await
        .context("Failed to get languages")?;
    log::info!("Languages :: {:#?}", languages);

    if !action.languages().is_empty() {
        log::info!("Validating language(s) :: {:?}", action.languages());

        action
            .validate_languages(&languages)
            .context("Failed to validate languages")?;
        log::info!("Language(s) validated");
    } else {
        log::info!("No languages provided, using all available languages");
    }

    log::info!("CodeQL :: {:?}", codeql);

    groupend!();

    let cwd = action
        .working_directory()
        .context("Failed to get working directory")?;
    let databases = cwd.join(".codeql");
    let sarif_output = databases.join("results");

    std::fs::create_dir_all(&sarif_output).context("Failed to create results directory")?;

    for language in action.languages() {
        group!(format!("Running {} extractor", language.language()));

        log::info!("Running extractor for language :: {}", language);

        let database_path = databases.join(format!("db-{}", language));
        log::info!("Database Path :: {:?}", database_path);
        if database_path.exists() {
            std::fs::remove_dir_all(&database_path).with_context(|| {
                format!("Failed to remove database directory {:?}", database_path)
            })?;
        }

        let sarif_path = sarif_output.join(format!("{}-results.sarif", language.language()));

        let mut database = CodeQLDatabase::init()
            .name(action.get_repository_name()?)
            .source(cwd.display().to_string())
            .path(database_path.display().to_string())
            .language(language.language())
            .build()
            .context("Failed to create database")?;

        log::info!("Creating database...");
        codeql
            .database(&database)
            .overwrite()
            .create()
            .await
            .context("Failed to create database")?;
        log::debug!("Created database :: {:?}", database);

        // TODO: Queries
        let queries = CodeQLQueries::from(format!(
            "{}/{}-queries",
            extractor_repo.owner.clone(),
            language.language()
        ));
        log::info!("Queries :: {:?}", queries);

        groupend!();
        group!(format!("Running {} analysis", language.language()));
        match codeql
            .database(&database)
            .queries(queries)
            .output(sarif_path)
            .analyze()
            .await
        {
            Ok(_) => {
                log::info!("Analysis complete");
            }
            Err(ghastoolkit::GHASError::SerdeError(e)) => {
                log::warn!("Failed to parse SARIF: {:?}", e);
            }
            Err(e) => {
                log::error!("Failed to analyze database: {:?}", e);
            }
        }

        // Reload the database to get analysis info
        database.reload()?;
        log::info!("CodeQL Database LoC :: {}", database.lines_of_code());

        log::info!("Analysis complete :: {:?}", database);
        groupend!();
    }

    log::info!("All databases created and analyzed");

    Ok(())
}
