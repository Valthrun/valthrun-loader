use std::path::Path;

use anyhow::Context;
use tokio::process::Command;
use windows_registry::LOCAL_MACHINE;

use crate::util::{self};

pub async fn execute_nal_fix(http: &reqwest::Client) -> anyhow::Result<()> {
    let path = util::get_downloads_path()?.join("nalfix.exe");

    util::download_file(
        http,
        "https://github.com/VollRagm/NalFix/releases/latest/download/NalFix.exe",
        &path,
    )
    .await
    .context("download file")?;

    util::invoke_command(&mut Command::new(path))
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

pub async fn disable_service(name: &str) -> anyhow::Result<()> {
    util::invoke_command(Command::new("sc").args(["stop", name])).await?;

    Ok(())
}

pub async fn add_defender_exclusion(path: &Path) -> anyhow::Result<()> {
    util::invoke_ps_command(&format!(
        "Add-MpPreference -ExclusionPath '{}' -ErrorAction SilentlyContinue",
        path.display()
    ))
    .await?;

    Ok(())
}
