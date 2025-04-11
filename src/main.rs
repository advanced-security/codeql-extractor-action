use std::path::PathBuf;

use anyhow::{Context, Result};
use ghactions::{ActionTrait, ToolCache, group, groupend};
use ghastoolkit::codeql::database::queries::CodeQLQueries;
use ghastoolkit::codeql::CodeQLLanguage;
use ghastoolkit::{CodeQL, CodeQLDatabase, CodeQLExtractor};
use log::{debug, info};

mod action;
mod extractors;

use action::Action;

#[tokio::main]
async fn main() -> Result<()> {
    let action = Action::init()?;
    debug!("Action :: {:?}", action);

    group!("Setting up Extractor");

    let client = octocrab::instance();

    let toolcache = ToolCache::new();
    debug!("ToolCache :: {:?}", toolcache);

    // Extractor
    let extractor_repos = action.extractor_repositories()?;
    info!("Extractor Repositories :: {:?}", extractor_repos);

    let extractor_path = PathBuf::from("./extractors");
    if !extractor_path.exists() {
        std::fs::create_dir(&extractor_path)
            .with_context(|| format!("Failed to create directory {:?}", extractor_path))?;
        info!("Created Extractor Directory :: {:?}", extractor_path);
    }

    let mut codeql_builder = CodeQL::init();
    let mut extractors = Vec::new();

    // Download and extract the extractor repositories
    for extractor_repo in extractor_repos {
        let extractor = extractors::fetch_extractor(
            &client,
            &extractor_repo,
            action.attestation(),
            &extractor_path,
        )
        .await
        .context("Failed to fetch extractor")?;
        log::info!("Extractor :: {:?}", extractor);

        codeql_builder = codeql_builder.search_path(extractor_path.clone());

        extractors.push((extractor_repo.clone(), CodeQLExtractor::load_path(extractor.clone())?));
    }

    let codeql = codeql_builder
            .build()
            .await
            .context("Failed to create CodeQL instance")?;

    log::info!("CodeQL :: {:?}", codeql);

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

    for (extractor_repo, extractor) in extractors.iter() {
        let language = CodeQLLanguage::from(extractor.name.clone());

        if !action.languages().is_empty() {
            if !action.languages().contains(&language) {
                log::info!("Skipping language :: {}", language);
                continue;
            }
        }

        let group = format!("Running `{}` extractor", language.language());
        group!(group);

        log::info!("Running extractor for language :: {}", language);

        let database_path = databases.join(format!("db-{}", language.language()));
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

        // TODO: Assumes the queries are in the same org
        let queries = CodeQLQueries::from(format!(
            "{}/{}-queries",
            extractor_repo.owner,
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
