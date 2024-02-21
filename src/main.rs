pub type Result<T = ()> = anyhow::Result<T>;

use std::cmp::Ordering;
use std::io::Write;

use fs_err as fs;
use git::TrunkRepo;
use once_cell::sync::Lazy;
use reqwest::Client;

use dist::get_dists;
use tempfile::tempdir;
use tracing::{error, info};
use trunk::{fetch_contrib_entries, trunk_toml::TrunkToml};

static CLIENT: Lazy<Client> = Lazy::new(Client::new);

static GH_PAT: Lazy<String> = Lazy::new(|| std::env::var("GH_PAT").unwrap());
static GH_EMAIL: Lazy<String> = Lazy::new(|| std::env::var("GH_EMAIL").unwrap());
static GH_USERNAME: Lazy<String> = Lazy::new(|| std::env::var("GH_USERNAME").unwrap());
static GH_AUTHOR: Lazy<String> = Lazy::new(|| std::env::var("GH_AUTHOR").unwrap());

/// Functions and types related to PGXN dist api
mod dist;
/// Functions and types related to managing git repos
mod git;
mod github;
/// Functions and types related to Trunk API
mod trunk;

#[tokio::main]
async fn main() -> Result {
    start_tracing();

    let tmp_dir = tempdir()?;
    let trunk_repo_path = tmp_dir.path().join("trunk");

    let mut trunk_repo = TrunkRepo::clone(&trunk_repo_path)?;
    info!("Cloned trunk to {}", trunk_repo_path.display());

    let (entries, dists) = tokio::try_join!(fetch_contrib_entries(), get_dists())?;

    for release in dists.recent.into_iter().rev() {
        // Check if extension is already in Trunk
        let maybe_trunk_entry = entries.iter().find(|entry| {
            entry.extension.name == release.dist
                || entry.extension.extension_name.as_deref() == Some(&release.dist)
        });

        if let Some(trunk_entry) = maybe_trunk_entry {
            println!("Already in Trunk: {}", trunk_entry.extension.name);
            if compare_by_semver(&release.version, &trunk_entry.extension.version)
                == Ordering::Greater
            {
                // Means pgxn has a more updated version of this extension compared to trunk
            } else {
                // Trunk has a more or equally updated version of this extension, so skip
                info!("{} is already up-to-date in Trunk, skipping.", trunk_entry.extension.name);
                continue;
            }
        }

        let metadata = release.get_metadata().await?;

        info!("Will open a PR for {} v{}", metadata.name, metadata.version);

        let branch_name = format!("pgxn-bridge/{}-{}", metadata.name, metadata.version);
        let commit_message = format!(
            "pgxn-bridge: publish {} v{}",
            metadata.name, metadata.version
        );

        let pull_request_description = github::build_description(&metadata)?;
        let trunk_toml = TrunkToml::build_from_pgxn_meta(metadata);
        let rendered_trunk_toml = toml::to_string_pretty(&trunk_toml)?;

        let directory = trunk_repo_path.join("contrib").join(&release.dist);
        fs::create_dir_all(&directory)?;
        let toml = directory.join("Trunk.toml");
        let mut toml = fs::File::create(toml)?;
        write!(toml, "{rendered_trunk_toml}")?;

        if let Err(err) = trunk_repo.commit_and_push(&commit_message, &branch_name) {
            error!("Commit failure: {err}");
            continue;
        }

        
        if let Err(err) = github::open_pull_request(&commit_message, &branch_name, &pull_request_description).await {
            error!("Failed to create pull request: {err}");
            continue;
        }
    }

    Ok(())
}

fn compare_by_semver(a: &str, b: &str) -> Ordering {
    let a_parts: Vec<i32> = a
        .split('.')
        .map(|p| p.parse::<i32>().unwrap_or(0))
        .collect();
    let b_parts: Vec<i32> = b
        .split('.')
        .map(|p| p.parse::<i32>().unwrap_or(0))
        .collect();

    let len = std::cmp::min(a_parts.len(), b_parts.len());

    for i in 0..len {
        match a_parts[i].cmp(&b_parts[i]) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            Ordering::Equal => continue,
        }
    }

    a_parts.len().cmp(&b_parts.len())
}

fn start_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
}