use std::{
    collections::HashMap,
    fmt::Display,
    io::{self, Cursor},
    ops::Not,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use fs_err as fs;
use serde::Deserialize;
use slicedisplay::SliceDisplay;
use tracing::info;
use zip::ZipArchive;

use crate::{Result, CLIENT};

/// Response from {PGXN}/stats/dist.json
#[derive(Debug, Clone, Deserialize)]
pub struct DistResponse {
    pub count: i64,
    pub releases: i64,
    pub recent: Vec<Release>,
}

#[derive(Debug, Clone, Deserialize)]
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
    fn download_url(&self) -> String {
        // Assumes the following spec: "/dist/{dist}/{version}/{dist}-{version}.zip"
        // TODO(vini): use the correct URL template from pgxn here

        let Self { dist, version, .. } = &self;

        let dist = dist.to_lowercase();

        format!("https://master.pgxn.org/dist/{dist}/{version}/{dist}-{version}.zip")
    }

    fn meta_url(&self) -> String {
        // Assumes the following spec: "/dist/{dist}/{version}/META.json"

        let Self { dist, version, .. } = &self;

        let dist = dist.to_lowercase();

        format!("https://master.pgxn.org/dist/{dist}/{version}/META.json")
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

    pub async fn get_metadata(&self) -> Result<MetaJson> {
        let url = self.meta_url();

        CLIENT
            .get(&url)
            .send()
            .await?
            .json()
            .await
            .with_context(|| format!("Failed to deserialize output of {url}"))
    }

    #[allow(unused)]
    pub async fn download_to(&self, target: &Path) -> Result<PathBuf> {
        let bytes = self.get_dist_zip().await?;

        let cursor = Cursor::new(&*bytes);
        let mut archive = ZipArchive::new(cursor)?;

        let mut root_dir = None;

        for idx in 0..archive.len() {
            let mut file = archive.by_index(idx)?;
            let Some(path) = file.enclosed_name() else {
                continue;
            };

            // The root directory will be the first one in the zip archive
            if root_dir.is_none() {
                root_dir = Some(target.join(path));
            }

            if file.is_dir() {
                fs::create_dir_all(target.join(path))?;
            } else {
                let target = target.join(path);
                if let Some(parent) = target.parent() {
                    if parent.exists().not() {
                        fs::create_dir_all(parent)?
                    }
                }
                let mut extracted_file = fs::File::create(target)?;
                io::copy(&mut file, &mut extracted_file)?;
            }
        }

        root_dir.with_context(|| "Expected a root directory to be found")
    }
}

pub async fn get_dists() -> anyhow::Result<DistResponse> {
    info!("Getting recent entries in PGXNs");

    let url = "https://master.pgxn.org/stats/dist.json";

    reqwest::get(url)
        .await?
        .json()
        .await
        .with_context(|| "Failed to deserialize response of /stats/dist.json")
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaJson {
    pub name: String,
    #[serde(rename = "abstract")]
    pub _abstract: String,
    pub description: Option<String>,
    pub version: String,
    pub date: String,
    pub maintainer: Maintainer,
    #[serde(rename = "release_status")]
    pub release_status: String,
    pub user: String,
    pub license: License,
    #[serde(default)]
    pub tags: Vec<String>,
    pub resources: Resources,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Maintainer {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum License {
    Simple(String),
    WithLink(HashMap<String, String>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Resources {
    pub bugtracker: Option<Bugtracker>,
    pub homepage: Option<String>,
    pub repository: Repository,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bugtracker {
    pub web: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Repository {
    pub url: String,
    pub web: String,
}

impl Display for Maintainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Maintainer::Single(single) => write!(f, "{single}"),
            Maintainer::Multiple(multiple) => {
                writeln!(f, "{}", multiple.display().terminator(' ', ' '))
            }
        }
    }
}
