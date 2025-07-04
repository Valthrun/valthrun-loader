use anyhow::Context;

use crate::{api, components, game, util};

pub async fn launch(http: &reqwest::Client, enhancer: components::Enhancer) -> anyhow::Result<()> {
    for artifact in enhancer.required_artifacts() {
        log::info!("Downloading {}", artifact.name());

        api::download_latest_artifact_version(http, artifact.slug(), artifact.file_name())
            .await
            .context("failed to download {}")?;
    }

    // TODO: Make it game-independent to also allow PUBG, for example
    if game::is_running()
        .await
        .context("failed to check if game is running")?
    {
        log::info!("Counter-Strike 2 is already running.");
    } else {
        log::info!("Counter-Strike 2 is not running.");

        if util::confirm_default("Do you want to launch the game?", true)? {
            log::info!("Waiting for Counter-Strike 2 to start");
            game::launch_and_wait()
                .await
                .context("failed to wait for cs2 to launch")?;
        }
    }

    util::invoke_ps_command(&format!(
        "Start-Process -FilePath '{}' -WorkingDirectory '{}'",
        util::get_downloads_path()?
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
