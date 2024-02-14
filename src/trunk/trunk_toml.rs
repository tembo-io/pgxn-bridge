use serde::Serialize;

use crate::dist::{License, MetaJson};

#[derive(Serialize, Debug)]
pub struct TrunkToml {
    pub extension: TomlExtensionData,
    pub build: TomlBuildInfo,
}

#[derive(Serialize, Debug)]
pub struct TomlExtensionData {
    pub name: String,
    pub extension_name: Option<String>,
    pub version: String,
    pub license: String,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct TomlBuildInfo {
    pub postgres_version: Option<String>,
    pub platform: String,
}

impl TrunkToml {
    pub fn build_from_pgxn_meta(meta: MetaJson) -> Self {
        let repository = meta.resources.repository.web;
        let homepage = meta
            .resources
            .homepage
            .unwrap_or_else(|| repository.clone());

        Self {
            extension: TomlExtensionData {
                name: meta.name,
                extension_name: None,
                version: meta.version,
                license: match meta.license {
                    License::Simple(license) => license,
                    License::WithLink(map) => map.into_iter().next().unwrap().0,
                },
                repository: Some(repository.clone()),
                description: Some(meta.description.unwrap_or(meta._abstract)),
                homepage: Some(homepage),
                documentation: Some(repository),
            },
            build: TomlBuildInfo {
                postgres_version: Some("15".into()),
                platform: "linux/amd64".into(),
            },
        }
    }
}
