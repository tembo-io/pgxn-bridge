pub type Result<T = ()> = anyhow::Result<T>;

use std::{cmp::Ordering, fs};

use once_cell::sync::Lazy;
use reqwest::Client;
use tempfile::tempdir;

use dist::get_dists;
use trunk::fetch_contrib_entries;

static CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

/// Functions and types related to PGXN dist api
mod dist;
mod trunk;

#[tokio::main]
async fn main() -> Result {
    let tmp_dir = tempdir()?;

    let (entries, dists) = tokio::try_join!(fetch_contrib_entries(), get_dists())?;

    for release in dists.recent {
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
                println!("Already updated in Trunk: {}", trunk_entry.extension.name);
                continue;
            }
        }

        let extracted_path = release.download_to(tmp_dir.path()).await?;
        println!("Extracted {}", extracted_path.display());

        // Create Dockerfile

        // TODO: open PR

        let _ = fs::remove_dir(extracted_path);
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
