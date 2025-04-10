use std::path::PathBuf;

use anyhow::{Context, Result};
use ghactions::{ActionTrait, ToolCache, group, groupend};
use ghastoolkit::{codeql::CodeQLLanguage, CodeQL, CodeQLDatabase};
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

    let codeql = CodeQL::init()
        .search_path(extractor)
        .build()
        .await
        .context("Failed to create CodeQL instance")?;
    log::info!("CodeQL :: {:?}", codeql);

    let languages = codeql.get_languages().await?;
    log::info!("Languages :: {:#?}", languages);

    if !action.languages().is_empty() {
        log::debug!("Validating languages");
       for lang in action.languages() {
            let qllang = CodeQLLanguage::from(lang.as_str());
            log::info!("Language :: {:?}", qllang);

            if !languages.contains(&qllang) {
                return Err(anyhow::anyhow!(
                    "Language {} is not supported by the extractor",
                    lang
                ));
            }
       }
    } else {
        log::info!("No languages provided, using all available languages");
    }

    groupend!();

    group!("Running extractor");

    for language in action.languages() {
        log::info!("Running extractor for language :: {}", language);

        let qllang = CodeQLLanguage::from(language.as_str());
        let database = CodeQLDatabase::init()
            .path(format!("./.codeql/db-{}", language))
            .language(qllang.language())
            .build()?;

        log::info!("Creating database...");
        codeql.database(&database)
            .overwrite()
            .create()
            .await?;
        log::info!("Created database :: {:?}", database);

        log::info!("Running analysis...");
        codeql.database(&database)
            .analyze()
            .await?;
        log::info!("Analysis complete :: {:?}", database);
    }

    groupend!();

    Ok(())
}
