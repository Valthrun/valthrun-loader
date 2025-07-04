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
