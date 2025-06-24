use anyhow::Context;
use thiserror::Error;
use tokio::process::Command;

use crate::{fixes, util};

#[derive(Debug, Error)]
pub enum MapDriverError {
    #[error("Nal Device is already in use")]
    DeviceNalInUse,
    #[error("Vulernable driver has been blocked")]
    DriverBlocklist,
    #[error("failed to spawn kdmapper process")]
    SpawnProcess(#[from] tokio::io::Error),
    #[error("failed to execute nal fix: {0}")]
    NalFixError(anyhow::Error),
    #[error("Unknown kdmapper error: {0}")]
    Unknown(String),
}

pub async fn map_driver() -> Result<bool, MapDriverError> {
    let downloads_path = util::get_downloads_path()
        .context("get downloads path")
        .unwrap();
    let kdmapper_path = downloads_path.join("kdmapper.exe");
    let driver_path = downloads_path.join("kernel_driver.sys");

    if let Err(e) = fixes::add_defender_exclusion(&kdmapper_path).await {
        log::warn!("Failed to add exclusion for Windows Defender: {:#}", e);
    };

    for service in ["faceit", "vgc", "vgk", "ESEADriver2"] {
        let _ = fixes::disable_service(service).await;
    }

    let output = util::invoke_command(Command::new(kdmapper_path).arg(driver_path)).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.contains("Device\\Nal is already in use") {
        return Err(MapDriverError::DeviceNalInUse);
    }

    if stdout.contains("0xc0000603") {
        return Err(MapDriverError::DriverBlocklist);
    }

    if stdout.contains("[+] success") && stdout.contains("0xcf000004") {
        return Ok(false);
    }

    if stdout.contains("[+] success") {
        return Ok(true);
    }

    return Err(MapDriverError::Unknown(stdout.to_string()));
}

pub async fn map_driver_handled(http: &reqwest::Client) -> anyhow::Result<()> {
    if let Err(e) = map_driver().await {
        match e {
            MapDriverError::DeviceNalInUse => {
                fixes::execute_nal_fix(http)
                    .await
                    .map_err(|e| MapDriverError::NalFixError(e))?;
                map_driver().await?;
            }
            MapDriverError::DriverBlocklist => {
                if let Err(e) = fixes::set_driver_blocklist(false) {
                    log::warn!("Failed to disable vulnerable driver blocklist: {:#}", e);
                }
                if let Err(e) = fixes::set_hvci(false) {
                    log::warn!("Failed to disable HVCI: {:#}", e);
                }

                log::warn!("The system must restart to continue changing system settings.");
                let should_restart = inquire::prompt_confirmation("Do you want to restart now?")
                    .context("prompt for restart")?;

                if should_restart {
                    util::schedule_restart().await.context("schedule restart")?;
                }

                std::process::exit(0);
            }
            e => anyhow::bail!(e),
        }
    };

    Ok(())
}
