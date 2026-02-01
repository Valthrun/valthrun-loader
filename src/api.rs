use std::path::PathBuf;

use reqwest::Client;

use crate::{
    components::{self, ArtifactSource},
    github, portal,
};

pub async fn download_latest_artifact_version(
    http: &Client,
    artifact: &components::Artifact,
) -> anyhow::Result<PathBuf> {
    match artifact.source() {
        ArtifactSource::Portal { .. } => {
            portal::download_latest_artifact_version(http, artifact).await
        }
        ArtifactSource::GithubRelease { .. } => {
            github::download_latest_artifact_version(http, artifact).await
        }
    }
}
