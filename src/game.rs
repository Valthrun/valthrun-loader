use std::path::Path;

use anyhow::Context;

use crate::util;

pub async fn is_running() -> anyhow::Result<bool> {
    let output =
        util::invoke_ps_command("Get-Process -Name cs2 -ErrorAction SilentlyContinue").await?;
    Ok(output.status.success())
}

pub async fn launch_and_wait() -> anyhow::Result<()> {
    util::invoke_ps_command("Start-Process 'steam://run/730'").await?;

    while !is_running().await? {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    Ok(())
}

// FIXME: Using PowerShell here is very much easier than working with the Windows API or some library. Might want to implement my own at some point..
// If any issues regarding launching as administrator arise, this function is ready to use
#[allow(dead_code)]
pub async fn create_and_run_task(name: &str, path: &Path) -> anyhow::Result<()> {
    let path = tokio::fs::canonicalize(path)
        .await
        .context("canonicalize path")?;

    let path = path.to_string_lossy();

    let script = format!(
        r#"$taskName = '{name}';
$trigger = New-ScheduledTaskTrigger -Once -At (Get-Date).Date.AddMinutes(1);
$action = New-ScheduledTaskAction -Execute $taskPath -WorkingDirectory '{path}';

Register-ScheduledTask -TaskName $taskName -Trigger $trigger -Action $action -User "$env:COMPUTERNAME\$env:USERNAME" -RunLevel Highest -Force -ErrorAction Stop | Out-Null;
Start-ScheduledTask -TaskName $taskName;
Start-Sleep -Seconds 2;

Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue | Out-Null;
"#
    );

    util::invoke_ps_command(&script).await?;

    todo!()
}
