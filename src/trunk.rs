pub mod trunk_toml;

use std::io::{Cursor, Read};
use std::ops::Not;

use crate::Result;
use anyhow::Context;
use flate2::read::GzDecoder;
use serde::Deserialize;
use tar::EntryType;

pub async fn fetch_contrib_entries() -> Result<Vec<ReducedTrunkToml>> {
    let url = "https://github.com/tembo-io/trunk/archive/refs/heads/main.tar.gz";
    let mut tomls = Vec::new();

    let trunk_archive = reqwest::get(url).await?.bytes().await?;

    // Decompress .gz
    let mut buf = Vec::with_capacity(trunk_archive.len() * 8);
    GzDecoder::new(&*trunk_archive).read_to_end(&mut buf)?;

    // buf now contains the contents of the tar archive
    let mut archive = tar::Archive::new(Cursor::new(buf));

    for maybe_entry in archive.entries()? {
        let mut entry = maybe_entry?;
        let header = entry.header();
        let entry_size = header.entry_size().unwrap_or(500);

        let EntryType::Regular = header.entry_type() else {
            continue;
        };

        let path = entry.path()?;

        let Some(parent) = path.parent() else {
            continue;
        };

        // Keep only paths within a directory within contrib
        // E.g.: `trunk-main/contrib/postgis/Trunk.toml`
        if parent
            .components()
            .rev()
            .nth(1)
            .map(|component| component.as_os_str() == "contrib")
            .unwrap_or(false)
            .not()
        {
            continue;
        }

        if path.file_name().unwrap() == "Trunk.toml" {
            let contents = {
                let mut buf = Vec::with_capacity(entry_size as usize);

                entry.read_to_end(&mut buf)?;
                String::from_utf8(buf).with_context(|| "Trunk.toml was not valid UTF-8")?
            };

            tomls.push(toml::from_str(&contents)?)
        }
    }

    Ok(tomls)
}

#[derive(Deserialize, Debug)]
pub struct ReducedTrunkToml {
    pub extension: ExtensionInfo,
}

#[derive(Deserialize, Debug)]
pub struct ExtensionInfo {
    pub name: String,
    pub extension_name: Option<String>,
    pub version: String,
}
