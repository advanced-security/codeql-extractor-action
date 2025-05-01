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

    group!("Setting up Extractor");

    let client = action.octocrab()?;

    let mut codeql = CodeQL::init()
        .build()
        .await
        .context("Failed to create CodeQL instance")?;

    if !codeql.is_installed().await {
        let codeql_version = action.codeql_version();
        log::info!("CodeQL not installed, installing {}...", codeql_version);
        codeql.install(&client, codeql_version).await?;
        log::info!("CodeQL installed");
    } else {
        log::info!("CodeQL already installed");
    }

    log::info!("CodeQL :: {:?}", codeql);

    // Extractor
    let extractor_repo = action.extractor_repository()?;
    info!("Extractor Repository :: {}", extractor_repo);

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

    codeql.append_search_path(extractor.display().to_string());

    let languages = codeql.get_languages().await?;
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

    groupend!();

    let databases = PathBuf::from("./.codeql");
    let sarif_output = databases.join("results");

    std::fs::create_dir_all(&sarif_output)?;

    for language in action.languages() {
        let group = format!("Running {} extractor", language.language());
        group!(group);

        log::info!("Running extractor for language :: {}", language);

        let database_path = databases.join(format!("db-{}", language));
        let sarif_path = sarif_output.join(format!("{}-results.sarif", language.language()));

        let database = CodeQLDatabase::init()
            .name(action.get_repository_name()?)
            .source(".".to_string())
            .path(database_path.display().to_string())
            .language(language.language())
            .build()?;

        log::info!("Creating database...");
        codeql.database(&database).overwrite().create().await?;
        log::info!("Created database :: {:?}", database);

        let queries = CodeQLQueries::from(format!(
            "{}/{}-queries",
            extractor_repo.owner.clone(),
            language.language()
        ));
        log::debug!("Queries :: {:?}", queries);

        log::info!("Running analysis...");
        if let Err(err) = codeql
            .database(&database)
            .queries(queries)
            .output(sarif_path)
            .analyze()
            .await
        {
            log::error!("Failed to analyze database: {:?}", err);
        }
        log::info!("Analysis complete :: {:?}", database);
        groupend!();
    }

    log::info!("All databases created and analyzed");

    Ok(())
}
