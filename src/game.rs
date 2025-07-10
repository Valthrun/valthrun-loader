use std::{pin::pin, time::Duration};

use crate::utils;

pub async fn is_running(name: &str) -> anyhow::Result<bool> {
    let output = utils::invoke_ps_command(&format!(
        "Get-Process -Name {name} -ErrorAction SilentlyContinue"
    ))
    .await?;
    Ok(output.status.success())
}

pub async fn wait_for_process(name: &str) -> anyhow::Result<()> {
    while !self::is_running(name).await? {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}

pub async fn launch_and_wait(url: &str, application_name: &str) -> anyhow::Result<()> {
    utils::invoke_ps_command(&format!("Start-Process '{url}'")).await?;

    let mut wait_process = pin!(self::wait_for_process(application_name));

    tokio::select! {
        _ = &mut wait_process => {
            return Ok(());
        },
        _ = tokio::time::sleep(Duration::from_secs(30)) => {
            /* timeout */
        }
    };

    log::warn!("Target {application_name} has not been launched automatically.");
    log::warn!("You may want to launch it manually in order to continue.");

    wait_process.await?;
    Ok(())
}
