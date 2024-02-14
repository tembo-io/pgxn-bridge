use once_cell::sync::Lazy;
use reqwest::Client;
use tempfile::tempdir;

use dist::get_dists;

static CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

/// Functions and types related to PGXN dist api
mod dist;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tmp_dir = tempdir()?;

    let dists = get_dists().await.unwrap();

    for release in dists.recent {
        release.download_to(tmp_dir.path()).await?;
    }

    Ok(())
}
