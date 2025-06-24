use anyhow::Context;

use crate::util;

pub fn compare_hashes(first: &str, second: &str) -> bool {
    let first = normalize_hash(first);
    let second = normalize_hash(second);

    first == second
}

pub fn normalize_hash(hash: &str) -> String {
    hash.to_lowercase().trim().to_string()
}

pub async fn get_stored_version_hash(artifact_slug: &str) -> anyhow::Result<Option<String>> {
    let path = util::get_versions_path()
        .context("get versions path")?
        .join(artifact_slug);

    if !path.exists() {
        return Ok(None);
    }

    let contents = tokio::fs::read_to_string(&path)
        .await
        .context("read version file contents")?;

    Ok(Some(normalize_hash(&contents)))
}

pub async fn set_stored_version_hash(artifact_slug: &str, hash: &str) -> anyhow::Result<()> {
    let path = util::get_versions_path()
        .context("get versions path")?
        .join(artifact_slug);

    tokio::fs::write(path, hash).await.context("write file")?;

    Ok(())
}
