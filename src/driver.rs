use anyhow::Context;
use thiserror::Error;
use tokio::process::Command;

use crate::{fixes, metrics, utils};

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
    let driver_path = downloads_path.join("driver_standalone.sys");

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
    let downloads_path = utils::get_downloads_path()
        .context("get downloads path")
        .unwrap();
    let kdmapper_path = downloads_path.join("kdmapper.exe");

    if fixes::is_defender_enabled()
        .await
        .context("check is defender enabled")?
        && !fixes::has_defender_exclusion(&kdmapper_path)
            .await
            .context("check defender exclusion")?
    {
        log::warn!("Windows Defender is enabled and there is no exclusion for the driver mapper.");

        if utils::confirm_default("Do you want to add an exclusion?", true)? {
            fixes::add_defender_exclusion(&kdmapper_path)
                .await
                .context("failed to add defender exclusion")?
        }
    }

    if let Err(e) = map_driver().await {
        metrics::add_record("map-error", format!("{e}"));
        match e {
            MapDriverError::DeviceNalInUse => {
                fixes::execute_nal_fix(http)
                    .await
                    .map_err(|e| MapDriverError::NalFixError(e))?;
                map_driver().await?;
            }
            MapDriverError::DriverBlocklist => {
                log::error!(
                    "Failed to load the driver due to the Vulnerable Driver Blocklist or HVCI being enabled."
                );

                if utils::confirm_default(
                    "Do you want to disable these Windows security features?",
                    true,
                )? {
                    if let Err(e) = fixes::set_driver_blocklist(false) {
                        metrics::add_record("error", format!("set-driver-blocklist: {e}"));
                        log::error!("Failed to disable vulnerable driver blocklist: {:#}", e);
                    }
                    if let Err(e) = fixes::set_hvci(false) {
                        metrics::add_record("error", format!("disable-hvci: {e}"));
                        log::error!("Failed to disable HVCI: {:#}", e);
                    }

                    log::info!("The system must restart to apply changes to the system settings.");
                    let should_restart =
                        utils::confirm_default("Do you want to restart now?", true)
                            .context("prompt for restart")?;

                    if should_restart {
                        log::info!("Restarting system");

                        utils::schedule_restart()
                            .await
                            .context("schedule restart")?;

                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }

                    anyhow::bail!("Please restart the system yourself and try again.");
                }
            }
            e => anyhow::bail!(e),
        }
    };

    Ok(())
}
