use anyhow::Context;

use crate::{api, driver, fixes, utils};

pub async fn map_driver(http: &reqwest::Client) -> anyhow::Result<()> {
    log::info!("Downloading Kernel Driver");

    api::download_latest_artifact_version(http, "kernel-driver", "kernel_driver.sys")
        .await
        .context("failed to download kernel driver")?;

    log::info!("Downloading KDMapper");

    utils::download_file(
        &http,
        "https://github.com/sinjs/kdmapper/releases/latest/download/kdmapper.exe",
        &utils::get_downloads_path()?.join("kdmapper.exe"),
    )
    .await
    .context("failed to download kdmapper")?;

    for service in [c"faceit", c"vgc", c"vgk", c"ESEADriver2"] {
        if fixes::is_service_running(service).context("check service running")?
            && utils::confirm_default(
                format!(
                    "Running service '{}' may interfere with the Valthrun Kernel Driver. Do you want to stop it?",
                    service.to_str()?
                ),
                true,
            )?
        {
            fixes::stop_service(service.to_str()?)
                .await
                .context("stop service")?;
        }
    }

    driver::ui_map_driver(&http)
        .await
        .context("failed to map driver")?;

    log::info!("Driver successfully mapped");

    Ok(())
}
