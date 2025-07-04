use anyhow::Context;
use thiserror::Error;
use tokio::process::Command;

use crate::{fixes, utils};

#[derive(Debug, Error)]
pub enum MapDriverError {
    #[error("vulernable driver has been blocked (0xc0000603)")]
    DriverBlocklist,
    #[error("failed to initialize the kernel driver logging system (0xcf000001)")]
    LogInitFailed,
    #[error("a function call initializing the kernel driver has failed (0xcf000002)")]
    PreInitFailed,
    #[error("failed to initialize the valthrun debug driver (0xcf000003)")]
    InitFailed,

    #[error("nal device is already in use")]
    DeviceNalInUse,
    #[error("failed to execute nal fix: {0:?}")]
    NalFixError(anyhow::Error),

    #[error("failed to spawn kdmapper process")]
    SpawnProcess(#[from] tokio::io::Error),
    #[error("Unknown kdmapper error: {0}")]
    Unknown(String),
}

pub async fn map_driver() -> Result<bool, MapDriverError> {
    let downloads_path = utils::get_downloads_path()
        .context("get downloads path")
        .unwrap();
    let kdmapper_path = downloads_path.join("kdmapper.exe");
    let driver_path = downloads_path.join("kernel_driver.sys");

    if let Err(e) = fixes::add_defender_exclusion(&kdmapper_path).await {
        log::warn!("Failed to add exclusion for Windows Defender: {:#}", e);
    };

    let output = utils::invoke_command(Command::new(kdmapper_path).arg(driver_path)).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    match stdout.as_ref() {
        s if s.contains("Device\\Nal is already in use") => Err(MapDriverError::DeviceNalInUse),
        s if s.contains("0xc0000603") => Err(MapDriverError::DriverBlocklist),
        s if s.contains("0xcf000001") => Err(MapDriverError::LogInitFailed),
        s if s.contains("0xcf000002") => Err(MapDriverError::PreInitFailed),
        s if s.contains("0xcf000003") => Err(MapDriverError::InitFailed),
        s if s.contains("[+] success") => Ok(!s.contains("0xcf000004")), // CSTATUS_DRIVER_ALREADY_LOADED
        s => Err(MapDriverError::Unknown(s.to_string())),
    }
}

pub async fn ui_map_driver(http: &reqwest::Client) -> anyhow::Result<()> {
    if let Err(e) = map_driver().await {
        match e {
            MapDriverError::DeviceNalInUse => {
                fixes::execute_nal_fix(http)
                    .await
                    .map_err(|e| MapDriverError::NalFixError(e))?;
                map_driver().await?;
            }
            MapDriverError::DriverBlocklist => {
                log::warn!(
                    "Failed to load the driver due to the Vulnerable Driver Blocklist or HVCI being enabled."
                );

                if utils::confirm_default(
                    "Do you want to disable these Windows security features?",
                    true,
                )? {
                    if let Err(e) = fixes::set_driver_blocklist(false) {
                        log::warn!("Failed to disable vulnerable driver blocklist: {:#}", e);
                    }
                    if let Err(e) = fixes::set_hvci(false) {
                        log::warn!("Failed to disable HVCI: {:#}", e);
                    }

                    log::warn!("The system must restart to apply changes to the system settings.");
                    let should_restart =
                        inquire::prompt_confirmation("Do you want to restart now?")
                            .context("prompt for restart")?;

                    if should_restart {
                        utils::schedule_restart()
                            .await
                            .context("schedule restart")?;
                    }

                    std::process::exit(0);
                }
            }
            e => anyhow::bail!(e),
        }
    };

    Ok(())
}
