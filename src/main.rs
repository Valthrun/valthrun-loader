use std::collections::HashMap;

use crate::api::download_latest_artifact_version;
use anyhow::{Context, Result};

mod api;
mod driver;
mod fixes;
mod game;
mod util;
mod version;

async fn real_main() -> Result<()> {
    let http = reqwest::Client::new();

    log::info!(
        "Valthrun Loader v{} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );
    log::info!("Current executable was built on {}", env!("BUILD_TIME"));

    // Download all artifacts from the Valthrun Portal
    log::info!("Starting download process...");
    let artifact_file_names = HashMap::from([
        ("cs2-overlay", "cs2_overlay.exe"),
        ("driver-interface-kernel", "driver_interface_kernel.dll"),
        ("kernel-driver", "kernel_driver.sys"),
    ]);
    for (artifact_slug, file_name) in artifact_file_names.iter() {
        download_latest_artifact_version(&http, artifact_slug, file_name)
            .await
            .with_context(|| {
                format!(
                    "failed to download latest artifact version for '{}'",
                    artifact_slug
                )
            })?;
    }

    // Download kdmapper
    log::info!("Downloading additional components...");
    util::download_file(
        &http,
        "https://github.com/sinjs/kdmapper/releases/latest/download/kdmapper.exe",
        &util::get_downloads_path()?.join("kdmapper.exe"),
    )
    .await
    .context("failed to download kdmapper")?;
    log::info!("All files downloaded and processed successfully.");

    // Map the driver
    log::info!("Mapping driver...");
    driver::map_driver_handled(&http)
        .await
        .context("failed to map driver with error handling")?;

    // Launch the game
    if game::is_running()
        .await
        .context("failed to check if game is running")?
    {
        log::info!("Counter-Strike 2 is already running.");
    } else {
        log::info!("Waiting for Counter-Strike 2 to start...");
        game::launch_and_wait()
            .await
            .context("failed to wait for cs2 to launch")?;
    }

    // Launch the overlay
    log::info!("Valthrun will now load. Have fun!");
    util::invoke_ps_command(&format!(
        "Start-Process -FilePath '{}' -WorkingDirectory '{}'",
        util::get_downloads_path()?
            .join("cs2_overlay.exe")
            .display(),
        std::env::current_exe()
            .context("get current exe")?
            .parent()
            .context("get parent path")?
            .display()
    ))
    .await
    .context("failed to start overlay")?;

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    if let Err(e) = real_main().await {
        log::error!("{:#}", e);
    }

    inquire::prompt_text("Press enter to continue...").expect("failed to prompt user");
}
