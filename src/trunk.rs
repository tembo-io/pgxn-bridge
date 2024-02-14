use std::{ops::Not, path::Path};

use anyhow::bail;
use tokio::process::Command;

use crate::dist::Release;

pub async fn build_project(release: &Release, source_dir: &Path) -> anyhow::Result<()> {
    let Release { dist, version, .. } = release;

    let exit_status = Command::new("trunk")
        .current_dir(source_dir)
        .args([
            "build",
            "--name",
            dist,
            "--version",
            version,
            "--pg-version",
            "15",
            "--output-path",
            &*source_dir.to_string_lossy(),
        ])
        .status()
        .await?;

    if exit_status.success().not() {
        bail!("Problem building {dist}-{version} for Pg15!");
    }

    Ok(())
}
