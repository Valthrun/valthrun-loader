use anyhow::Context;

use crate::{api, components, driver, fixes, utils};

pub async fn map_driver(http: &reqwest::Client) -> anyhow::Result<()> {
    log::info!("Checking for interfering services");

    for service in [c"faceit", c"vgc", c"vgk", c"ESEADriver2"] {
        if fixes::is_service_running(service).context("check service running")? {
            log::error!(
                "The service '{}' will cause the driver mapping to fail. In order to proceed, you need to stop this service.",
                service.to_str()?
            );

            if utils::confirm_default("Do you want to stop this service?", true)? {
                fixes::stop_service(service.to_str()?)
                    .await
                    .context("stop service")?;
            }
        }
    }

    api::download_latest_artifact_version(http, components::Artifact::KernelDriver)
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

    driver::ui_map_driver(&http)
        .await
        .context("failed to map driver")?;

    log::info!("Driver successfully mapped");

    Ok(())
}
