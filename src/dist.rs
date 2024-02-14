use std::{
    io::{self, Cursor},
    ops::Not,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::CLIENT;

/// Response from {PGXN}/stats/dist.json
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistResponse {
    pub count: i64,
    pub releases: i64,
    pub recent: Vec<Release>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    pub dist: String,
    pub version: String,
    #[serde(rename = "abstract")]
    pub description: String,
    pub date: String,
    pub user: String,
    pub user_name: String,
}

impl Release {
    pub fn download_url(&self) -> String {
        // Assumes the following spec: "/dist/{dist}/{version}/{dist}-{version}.zip"
        // TODO(vini): use the correct URL template from pgxn here

        let Self { dist, version, .. } = &self;

        format!("https://master.pgxn.org/dist/{dist}/{version}/{dist}-{version}.zip")
    }

    /// Get this distribution's zip archive as bytes
    async fn get_dist_zip(&self) -> anyhow::Result<bytes::Bytes> {
        let Self { dist, version, .. } = self;
        let url = self.download_url();

        let response = CLIENT.get(url).send().await?;

        if response.status().is_success().not() {
            let error_msg = response.text().await?;
            bail!("Failed to fetch {dist}-{version}: {error_msg}")
        }

        response.bytes().await.map_err(Into::into)
    }

    pub async fn download_to(&self, target: &Path) -> anyhow::Result<PathBuf> {
        let bytes = self.get_dist_zip().await?;

        let cursor = Cursor::new(&*bytes);
        let mut archive = ZipArchive::new(cursor)?;

        for idx in 0..archive.len() {
            let mut file = archive.by_index(idx)?;
            let Some(path) = file.enclosed_name() else {
                continue;
            };

            if file.is_dir() {
                println!("Created dir {:?}", path);
                fs::create_dir_all(target.join(path))?;
            } else {
                if let Some(parent) = path.parent() {
                    if parent.exists().not() {
                        fs::create_dir_all(parent)?
                    }
                }
                let mut extracted_file = fs::File::create(path)?;
                print!("Extracting file {:?}", path);
                io::copy(&mut file, &mut extracted_file)?;
                println!(".. done!");
            }
        }

        todo!()
    }
}

pub async fn get_dists() -> anyhow::Result<DistResponse> {
    let url = "https://master.pgxn.org/stats/dist.json";

    reqwest::get(url)
        .await?
        .json()
        .await
        .with_context(|| "Failed to deserialize response of /stats/dist.json")
}
