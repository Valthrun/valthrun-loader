use anyhow::Context;
use futures::StreamExt;

use crate::{api, utils};

#[derive(Debug, Clone)]
struct Update(api::Version);

impl Update {
    pub fn download_url(&self) -> String {
        format!(
            "https://valth.run/api/artifacts/{}/{}/{}/download",
            self.0.artifact, self.0.track, self.0.id
        )
    }

    pub async fn download_and_install(&self, http: &reqwest::Client) -> anyhow::Result<()> {
        let mut stream = http
            .get(self.download_url())
            .send()
            .await
            .context("send request")?
            .error_for_status()?
            .bytes_stream();

        let file = tempfile::NamedTempFile::new().context("create tempfile")?;
        let mut buf = std::io::BufWriter::new(&file);

        while let Some(item) = stream.next().await {
            std::io::copy(&mut item?.as_ref(), &mut buf).context("copy data")?;
        }

        log::debug!("Downloaded update to {}", file.path().display());

        self_replace::self_replace(file.path()).context("replace self")?;

        Ok(())
    }
}

async fn check_for_updates(http: &reqwest::Client) -> anyhow::Result<Option<Update>> {
    log::debug!("Checking for updates");

    if cfg!(debug_assertions) {
        log::debug!("Running in debug version, skipping update check");
        return Ok(None);
    }

    let latest_version = api::get_latest_artifact_track_version(http, "valthrun-loader", "win32")
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
    let Some(update) = check_for_updates(http).await.context("check for updates")? else {
        return Ok(());
    };

    log::info!("A new update for the loader is available.");
    log::info!(
        "  Installed version: {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );
    log::info!(
        "  Available version: {} ({})",
        update.0.version,
        update.0.version_hash
    );

    if !utils::confirm_default(
        "Do you want to download and install the latest version?",
        true,
    )? {
        return Ok(());
    }

    update
        .download_and_install(http)
        .await
        .context("download and install update")?;

    log::debug!("Update installed successfully. Restarting process");

    restart().await;
}

async fn restart() -> ! {
    async fn restart_internal() -> anyhow::Result<()> {
        let current_exe = std::env::current_exe()?;

        if utils::is_console_invoked() {
            // If the loader is invoked from the command line, just spawn the process with the stdio inherited
            // and wait for it to exit.

            let exit = std::process::Command::new(current_exe)
                .args(std::env::args_os().skip(1))
                .spawn()?
                .wait()?
                .code()
                .unwrap_or(1);

            std::process::exit(exit);
        } else {
            // If the loader is invoked normally, use Start-Process since that will not break is_console_invoked().
            // Arguments do not matter in this case, and the current process exits after spawning the new one.

            utils::invoke_ps_command(&format!(
                "Start-Process -FilePath '{}'",
                current_exe.display(),
            ))
            .await?;

            std::process::exit(0)
        }
    }

    if let Err(e) = restart_internal()
        .await
        .context("Failed to restart the loader")
    {
        log::error!("{:#}", e);
        log::error!("Please restart the loader manually.");

        if utils::is_console_invoked() {
            utils::console_pause();
        }
    }

    std::process::exit(0);
}
