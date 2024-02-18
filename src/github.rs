use anyhow::bail;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde_json::json;

use std::{fmt::Write, ops::Not};

use crate::{dist::MetaJson, Result, CLIENT, GH_PAT};

pub fn build_description(metadata: &MetaJson) -> Result<String> {
    let MetaJson {
        name,
        _abstract,
        description,
        version,
        date,
        maintainer,
        ..
    } = &metadata;

    let mut buf = String::with_capacity(256);

    writeln!(buf, "Note: this PR was auto-generated by [pgxn-bridge](https://github.com/tembo-io/pgxn-bridge), see [{name} in PGXN](https://pgxn.org/dist/{name}/)\n")?;
    writeln!(buf, "Version {version}, published {date}\n")?;
    writeln!(
        buf,
        "Description: {}\n",
        description.as_deref().unwrap_or(_abstract)
    )?;
    writeln!(buf, "Maintainer: {maintainer}")?;

    Ok(buf)
}

pub async fn open_pull_request(title: &str, branch_name: &str, description: &str) -> Result {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("request"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("token {}", &*GH_PAT))?,
    );

    let response = CLIENT
        .post("https://api.github.com/repos/vrmiguel/trunk/pulls")
        .json(&json!({
            "title": title,
            "head": branch_name,
            "base": "main",
            "body": description
        }))
        .headers(headers)
        .send()
        .await?;

    if response.status().is_success().not() {
        let error_msg = response.text().await?;
        bail!("Failed to open pull request: {error_msg}");
    }

    Ok(())
}
