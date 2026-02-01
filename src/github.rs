use std::path::PathBuf;

use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    components::{self, ArtifactSource},
    utils, version,
};

#[derive(Debug, Serialize, Deserialize)]
struct Release {
    id: usize,
    tag_name: String,
    assets: Vec<Asset>,
}

impl Release {
    pub fn version_hash(&self) -> String {
        format!("github-{}", self.id)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub async fn download_latest_artifact_version(
    http: &Client,
    artifact: &components::Artifact,
) -> anyhow::Result<PathBuf> {
    let ArtifactSource::GithubRelease { owner, repo } = artifact.source() else {
        anyhow::bail!("artifact does not have a github source");
    };

    let release: Release = http
        .get(format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        ))
        .header("User-Agent", "valthrun-loader")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let stored_hash = version::get_stored_version_hash(artifact.slug())
        .await
        .context("get stored version hash")?;

    let output_path = utils::get_downloads_path()
        .context("get downloads path")?
        .join(artifact.file_name());

    let version_hash = release.version_hash();

    let should_download = !output_path.is_file()
        || stored_hash.is_none_or(|hash| !version::compare_hashes(&hash, &version_hash));

    if should_download {
        if output_path.is_file() {
            log::info!(
                "{} is outdated. Downloading new version {} from GitHub.",
                artifact.name(),
                release.tag_name,
            );
        } else {
            log::info!(
                "{} not found locally. Downloading version {} from GitHub.",
                artifact.name(),
                release.tag_name,
            );
        }

        let asset = release
            .assets
            .into_iter()
            .find(|a| a.name == artifact.file_name())
            .context("asset with file name not found in latest release")?;

        utils::download_file(http, &asset.browser_download_url, &output_path)
            .await
            .context("download file")?;

        version::set_stored_version_hash(artifact.slug(), &version_hash)
            .await
            .context("set stored version hash")?;
    } else {
        log::info!(
            "Latest version of {} found locally. Skipping download.",
            artifact.name()
        );
    }

    Ok(output_path)
}
