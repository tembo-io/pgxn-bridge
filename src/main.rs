use std::fs;

use once_cell::sync::Lazy;
use reqwest::Client;
use tempfile::tempdir;

use dist::get_dists;

use crate::trunk::build_project;

static CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

/// Functions and types related to PGXN dist api
mod dist;
mod trunk;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tmp_dir = tempdir()?;

    let dists = get_dists().await.unwrap();

    for release in dists.recent {
        // TODO: check if extension is already in Trunk
        let extracted_path = release.download_to(tmp_dir.path()).await?;
        println!("Extracted {}", extracted_path.display());

        if let Err(err) = build_project(&release, &extracted_path).await {
            eprintln!("Error: {err}");
            continue;
        }

        // TODO: open PR

        let _ = fs::remove_dir(extracted_path);
    }

    Ok(())
}
