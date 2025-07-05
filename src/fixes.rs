use std::{ffi::CStr, path::Path};

use anyhow::Context;
use tokio::process::Command;
use windows::{
    Win32::{
        Foundation::ERROR_SERVICE_DOES_NOT_EXIST,
        System::Services::{
            OpenSCManagerA, OpenServiceA, QueryServiceStatus, SC_MANAGER_CONNECT,
            SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_STATUS,
        },
    },
    core::PCSTR,
};
use windows_registry::LOCAL_MACHINE;

use crate::utils::{self};

pub async fn execute_nal_fix(http: &reqwest::Client) -> anyhow::Result<()> {
    let path = utils::get_downloads_path()?.join("nalfix.exe");

    utils::download_file(
        http,
        "https://github.com/VollRagm/NalFix/releases/latest/download/NalFix.exe",
        &path,
    )
    .await
    .context("download file")?;

    utils::invoke_command(&mut Command::new(path))
        .await
        .context("execute command")?;

    Ok(())
}

pub fn set_hvci(enabled: bool) -> windows_registry::Result<()> {
    let key = LOCAL_MACHINE.create(r"System\CurrentControlSet\Control\DeviceGuard\Scenarios")?;
    key.set_u32("HypervisorEnforcedCodeIntegrity", enabled.into())?;
    Ok(())
}

pub fn set_driver_blocklist(enabled: bool) -> windows_registry::Result<()> {
    let key = LOCAL_MACHINE.create(r"System\CurrentControlSet\Control\CI\Config")?;
    key.set_u32("VulnerableDriverBlocklistEnable", enabled.into())?;
    Ok(())
}

pub fn is_service_running(name: &CStr) -> anyhow::Result<bool> {
    let running = unsafe {
        let hsc_manager = OpenSCManagerA(None, None, SC_MANAGER_CONNECT | SERVICE_QUERY_STATUS)
            .context("OpenSCManagerA")?;

        let service = match OpenServiceA(
            hsc_manager,
            PCSTR(name.as_ptr() as *const u8),
            SERVICE_QUERY_STATUS,
        ) {
            Ok(handle) => handle,
            Err(error) if error.code() == ERROR_SERVICE_DOES_NOT_EXIST.to_hresult() => {
                return Ok(false);
            }
            Err(error) => {
                anyhow::bail!(
                    "failed to open service '{}': {}",
                    name.to_string_lossy(),
                    error
                )
            }
        };

        let mut status = SERVICE_STATUS::default();
        QueryServiceStatus(service, &mut status).context("QueryServiceStatus")?;

        status.dwCurrentState == SERVICE_RUNNING
    };

    Ok(running)
}

pub async fn stop_service(name: &str) -> anyhow::Result<()> {
    utils::invoke_command(Command::new("sc").args(["stop", name])).await?;

    Ok(())
}

fn parse_powershell_boolean(output: impl AsRef<str>) -> anyhow::Result<bool> {
    let output = output.as_ref();
    if output.contains("True") {
        Ok(true)
    } else if output.contains("False") {
        Ok(false)
    } else {
        anyhow::bail!(
            "failed to parse command output: (expected powershell boolean, got: '{}')",
            output
        )
    }
}

pub async fn is_defender_enabled() -> anyhow::Result<bool> {
    let output =
        utils::invoke_ps_command(&format!("(Get-MpComputerStatus).RealTimeProtectionEnabled"))
            .await?;

    let output = String::from_utf8_lossy(&output.stdout);

    parse_powershell_boolean(output)
}

pub async fn has_defender_exclusion(path: &Path) -> anyhow::Result<bool> {
    let output = utils::invoke_ps_command(&format!(
        "(Get-MpPreference).ExclusionPath -contains '{}'",
        path.display()
    ))
    .await?;

    let output = String::from_utf8_lossy(&output.stdout);

    parse_powershell_boolean(output)
}

pub async fn add_defender_exclusion(path: &Path) -> anyhow::Result<()> {
    utils::invoke_ps_command(&format!(
        "Add-MpPreference -ExclusionPath '{}' -ErrorAction SilentlyContinue",
        path.display()
    ))
    .await?;

    Ok(())
}
