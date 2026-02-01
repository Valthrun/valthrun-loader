use std::{env, fs, path::PathBuf, process::Command, time::Duration};

use anyhow::Context;
use futures::StreamExt;

use crate::{CommandExecuteUpdate, metrics, portal, utils};

#[derive(Debug, Clone)]
struct Update(portal::Version);

impl Update {
    pub fn download_url(&self) -> String {
        format!(
            "https://valth.run/api/artifacts/{}/{}/{}/download",
            self.0.artifact, self.0.track, self.0.id
        )
    }

    pub async fn download_update(&self, http: &reqwest::Client) -> anyhow::Result<PathBuf> {
        if let Ok(mock_updater) = env::var("VTL_UPDATER_EXE") {
            log::debug!("Skipping update download. Using mock file at {mock_updater}");
            return Ok(PathBuf::from(mock_updater));
        }

        let mut stream = http
            .get(self.download_url())
            .send()
            .await
            .context("send request")?
            .error_for_status()?
            .bytes_stream();

        let file = tempfile::NamedTempFile::new().context("create tempfile")?;
        {
            let mut buf = std::io::BufWriter::new(&file);

            while let Some(item) = stream.next().await {
                std::io::copy(&mut item?.as_ref(), &mut buf).context("copy data")?;
            }

            log::debug!("Downloaded update to {}", file.path().display());
        }
        let (_file, file_path) = file.keep().context("keep update")?;
        Ok(file_path)
    }
}

async fn check_for_updates(http: &reqwest::Client) -> anyhow::Result<Option<Update>> {
    log::debug!("Checking for updates");

    if cfg!(debug_assertions) {
        log::debug!("Running in debug version, skipping update check");
        return Ok(None);
    }

    let latest_version =
        portal::get_latest_artifact_track_version(http, "valthrun-loader", "win32")
            .await
            .context("failed to get latest version")?;

    let has_update = env!("GIT_HASH") != latest_version.version_hash;

    log::debug!(
        "Has update: {has_update} (Latest: {}, Current: {})",
        latest_version.version_hash,
        env!("GIT_HASH")
    );

    Ok(if has_update {
        Some(Update(latest_version))
    } else {
        None
    })
}

pub async fn ui_updater(http: &reqwest::Client) -> anyhow::Result<()> {
    let update = match check_for_updates(http).await {
        Ok(Some(update)) => update,
        Ok(None) => {
            return Ok(());
        }
        Err(error) => {
            log::warn!("Failed to check for loader updates: {error}");
            return Ok(());
        }
    };

    log::warn!("Your version of the Valthrun loader is outdated.");
    if !utils::confirm_default(
        &format!(
            "Do you want to update to v{} (#{})?",
            update.0.version, update.0.version_hash
        ),
        true,
    )? {
        return Ok(());
    }

    log::info!("Downloading update...");
    let updater = update
        .download_update(http)
        .await
        .context("download and install update")?;

    log::info!("Update downloaded successfully. Installing and restarting...");
    let _ = tokio::time::sleep(Duration::from_secs(1)).await;

    let current_file = env::current_exe().context("current exe")?;
    let _update_process = Command::new(updater)
        .arg("execute-update")
        .arg("--target-file")
        .arg(format!("{}", current_file.display()))
        .arg("--source-version")
        .arg(env!("CARGO_PKG_VERSION"))
        .arg("--source-hash")
        .arg(env!("GIT_HASH"))
        .arg("--console-invoked")
        .arg(format!("{}", utils::is_console_invoked()))
        .spawn()
        .context("invoking updater")?;

    metrics::add_record("self-update", "execute");
    metrics::shutdown();

    /* exit early and let the update do it's job */
    std::process::exit(0);
}

pub async fn execute(command: &CommandExecuteUpdate) -> anyhow::Result<()> {
    log::info!(
        "Updating from {} (#{}) to {} (#{})",
        command.source_version,
        command.source_hash,
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );

    fs::copy(
        &env::current_exe().context("current exe")?,
        &command.target_file,
    )
    .context("copy new version")?;

    if let Err(error) = self_replace::self_delete() {
        log::warn!("Failed to mark the update as delete after exit: {error}");
    }

    if command.console_invoked {
        log::info!("Update successful. You can not run the Valthrun loader again.");
    } else {
        log::info!("Update successful. Launching Valthrun loader.");
        let _target = Command::new(&command.target_file)
            .spawn()
            .context("launch updated loader")?;
    }

    Ok(())
}
