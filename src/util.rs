use std::{
    path::{Path, PathBuf},
    process::Output,
};

use anyhow::Context;
use futures::StreamExt;
use tokio::process::Command;
use windows::Win32::System::Console::GetConsoleProcessList;

pub async fn invoke_ps_command(command: &str) -> tokio::io::Result<Output> {
    self::invoke_command(Command::new("powershell").args(&["-Command", &command])).await
}

pub async fn invoke_command(command: &mut Command) -> tokio::io::Result<Output> {
    log::trace!("Invoking: {:?}", command);
    let output = command.output().await?;
    log::trace!("Command output:");
    log::trace!(
        "  exit code: 0x{:X}",
        output.status.code().unwrap_or_default()
    );
    if output.stdout.len() > 0 {
        log::trace!("  stdout:",);
        for line in String::from_utf8_lossy(output.stdout.as_slice()).lines() {
            log::trace!("    {}", line.trim_end());
        }
    } else {
        log::trace!("  stdout: (empty)",);
    }
    if output.stderr.len() > 0 {
        log::trace!("  stderr:",);
        for line in String::from_utf8_lossy(output.stderr.as_slice()).lines() {
            log::trace!("    {}", line.trim_end());
        }
    } else {
        log::trace!("  stderr: (empty)",);
    }
    Ok(output)
}

pub fn get_data_path() -> anyhow::Result<PathBuf> {
    let path = std::env::current_exe()
        .context("get current exe")?
        .parent()
        .context("get parent path")?
        .join(".vtl");

    std::fs::create_dir_all(&path)?;

    Ok(path)
}

pub fn get_downloads_path() -> anyhow::Result<PathBuf> {
    let path = get_data_path().context("get data path")?.join("downloads");

    std::fs::create_dir_all(&path)?;

    Ok(path)
}

pub fn get_versions_path() -> anyhow::Result<PathBuf> {
    let path = get_data_path().context("get data path")?.join("versions");

    std::fs::create_dir_all(&path)?;

    Ok(path)
}

pub async fn download_file(
    http: &reqwest::Client,
    url: impl reqwest::IntoUrl,
    path: &Path,
) -> anyhow::Result<()> {
    log::debug!(
        "Downloading file from {} to {}",
        url.as_str(),
        path.display()
    );

    let mut stream = http
        .get(url)
        .send()
        .await
        .context("send request")?
        .error_for_status()?
        .bytes_stream();

    let file = tokio::fs::File::create(&path)
        .await
        .context("create file")?;
    let mut buf = tokio::io::BufWriter::new(file);

    while let Some(item) = stream.next().await {
        tokio::io::copy(&mut item?.as_ref(), &mut buf)
            .await
            .context("copy data")?;
    }

    Ok(())
}

pub async fn schedule_restart() -> anyhow::Result<()> {
    invoke_command(Command::new("shutdown").args(["/r", "/t", "0"])).await?;

    std::process::exit(1);
}

pub fn is_console_invoked() -> bool {
    let mut result: [u32; 128] = [0u32; 128];

    let console_count = unsafe { GetConsoleProcessList(&mut result) };
    console_count > 1
}

pub fn console_pause() {
    inquire::prompt_text("Press enter to continue...").expect("failed to prompt user");
}

pub fn confirm_default(message: impl AsRef<str>, default: bool) -> anyhow::Result<bool> {
    inquire::Confirm::new(message.as_ref())
        .with_default(default)
        .prompt()
        .context("prompt user")
}
