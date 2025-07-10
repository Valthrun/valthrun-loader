use std::path::PathBuf;

use anyhow::Context;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    components,
    utils::{self},
    version::{self},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactResponse {
    pub artifact: Artifact,
    pub tracks: Vec<Track>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub default_track: Uuid,
    pub sort_index: String,
    pub private: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrackResponse {
    pub artifact: Artifact,
    pub track: Track,
    pub versions: Vec<Version>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: Uuid,
    pub artifact: Uuid,
    pub slug: String,
    pub name: String,
    pub last_version: Uuid,
    pub sort_index: String,
    pub private: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionResponse {
    pub artifact: Artifact,
    pub track: Track,
    pub version: Version,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: Uuid,
    pub artifact: Uuid,
    pub track: Uuid,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub version_hash: String,
    pub file_size: String,
    pub file_name: String,
    pub file_extension: String,
    pub file_type: String,
    pub requires_signing: bool,
    pub download_count: String,
}

pub async fn get_artifact(http: &Client, artifact_slug: &str) -> reqwest::Result<ArtifactResponse> {
    let response = http
        .get(format!("https://valth.run/api/artifacts/{}", artifact_slug))
        .send()
        .await?
        .error_for_status()?
        .json::<ArtifactResponse>()
        .await?;

    Ok(response)
}

pub async fn get_track(
    http: &Client,
    artifact_slug: &str,
    track_slug: &str,
) -> reqwest::Result<TrackResponse> {
    let response = http
        .get(format!(
            "https://valth.run/api/artifacts/{}/{}",
            artifact_slug, track_slug
        ))
        .send()
        .await?
        .error_for_status()?
        .json::<TrackResponse>()
        .await?;

    Ok(response)
}

pub async fn get_latest_artifact_version(
    http: &Client,
    artifact_slug: &str,
) -> anyhow::Result<Version> {
    let artifact = get_artifact(http, artifact_slug).await?.artifact;

    get_latest_artifact_track_version(http, artifact_slug, &artifact.default_track.to_string())
        .await
}

pub async fn get_latest_artifact_track_version(
    http: &Client,
    artifact_slug: &str,
    track_slug: &str,
) -> anyhow::Result<Version> {
    let track_response = get_track(http, &artifact_slug, &track_slug).await?;

    let latest_version = track_response
        .versions
        .iter()
        .find(|v| v.id == track_response.track.last_version)
        .cloned()
        .context("failed find latest version")?;

    Ok(latest_version)
}

pub async fn download_latest_artifact_version(
    http: &Client,
    artifact: &components::Artifact,
) -> anyhow::Result<PathBuf> {
    let latest_version = get_latest_artifact_version(http, artifact.slug())
        .await
        .context("get latest artifact version")?;

    let stored_hash = version::get_stored_version_hash(artifact.slug())
        .await
        .context("get stored version hash")?;

    let output_path = utils::get_downloads_path()
        .context("get downloads path")?
        .join(artifact.file_name());

    let should_download = !output_path.is_file()
        || stored_hash
            .is_none_or(|hash| !version::compare_hashes(&hash, &latest_version.version_hash));

    if should_download {
        if output_path.is_file() {
            log::info!(
                "{} is outdated. Downloading new version {} (#{}).",
                artifact.name(),
                latest_version.version,
                latest_version.version_hash
            );
        } else {
            log::info!(
                "{} not found locally. Downloading version {} (#{}).",
                artifact.name(),
                latest_version.version,
                latest_version.version_hash
            );
        }

        utils::download_file(
            http,
            format!(
                "https://valth.run/api/artifacts/{}/{}/{}/download",
                artifact.slug(),
                latest_version.track,
                latest_version.id
            ),
            &output_path,
        )
        .await
        .context("download file")?;

        version::set_stored_version_hash(artifact.slug(), &latest_version.version_hash)
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
