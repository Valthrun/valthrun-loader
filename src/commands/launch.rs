use std::time::Duration;

use anyhow::Context;

use crate::{api, components, game, utils};

const APP_CS2_NAME: &str = "cs2";
const APP_CS2_URL: &str = "steam://run/730";

pub async fn launch(http: &reqwest::Client, enhancer: components::Enhancer) -> anyhow::Result<()> {
    for artifact in enhancer.required_artifacts() {
        api::download_latest_artifact_version(http, &artifact)
            .await
            .context("failed to download {}")?;
    }

    if game::is_running(APP_CS2_NAME)
        .await
        .context("failed to check if game is running")?
    {
        log::info!("Counter-Strike 2 is already running.");
    } else {
        log::info!("Counter-Strike 2 is not running.");

        if utils::confirm_default("Do you want to launch the game?", true)? {
            log::info!("Waiting for Counter-Strike 2 to start");
            game::launch_and_wait(APP_CS2_NAME, APP_CS2_URL)
                .await
                .context("failed to wait for cs2 to launch")?;

            /* wait 15 more seconds for CS2 to load */
            log::debug!("Waiting 15 more seconds for CS2 to properly initialize.");
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    }

    utils::invoke_ps_command(&format!(
        "Start-Process -FilePath '{}' -WorkingDirectory '{}'",
        utils::get_downloads_path()?
            .join(enhancer.artifact_to_execute().file_name())
            .display(),
        std::env::current_exe()
            .context("get current exe")?
            .parent()
            .context("get parent path")?
            .display()
    ))
    .await
    .context("failed to start overlay")?;

    log::info!("Valthrun will now load. Have fun!");

    Ok(())
}
