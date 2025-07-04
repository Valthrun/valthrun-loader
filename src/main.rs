use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

mod api;
mod commands;
mod driver;
mod fixes;
mod game;
mod ui;
mod updater;
mod util;
mod version;

#[derive(Parser, Debug)]
pub struct AppArgs {
    /// Enable verbose logging ($env:RUST_LOG="trace")
    #[clap(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<AppCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Artifact {
    Cs2Overlay,
    Cs2RadarClient,
    DriverInterfaceKernel,
    KernelDriver,
}

impl Artifact {
    pub const fn name(&self) -> &'static str {
        match self {
            Artifact::Cs2Overlay => "CS2 Overlay",
            Artifact::Cs2RadarClient => "CS2 Radar Client",
            Artifact::DriverInterfaceKernel => "Driver Interface Kernel",
            Artifact::KernelDriver => "Kernel Driver",
        }
    }

    pub const fn slug(&self) -> &'static str {
        match self {
            Artifact::Cs2Overlay => "cs2-overlay",
            Artifact::Cs2RadarClient => "cs2-radar-client",
            Artifact::DriverInterfaceKernel => "driver-interface-kernel",
            Artifact::KernelDriver => "kernel-driver",
        }
    }

    pub const fn file_name(&self) -> &'static str {
        match self {
            Artifact::Cs2Overlay => "cs2_overlay.exe",
            Artifact::Cs2RadarClient => "cs2_radar_client.exe",
            Artifact::DriverInterfaceKernel => "driver_interface_kernel.dll",
            Artifact::KernelDriver => "kernel_driver.sys",
        }
    }
}

#[derive(ValueEnum, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "kebab-case")]
pub enum Enhancer {
    Cs2Overlay,
    Cs2StandaloneRadar,
}

impl Enhancer {
    pub const fn required_artifacts(&self) -> &'static [&'static Artifact] {
        match self {
            Enhancer::Cs2Overlay => &[&Artifact::Cs2Overlay, &Artifact::DriverInterfaceKernel],
            Enhancer::Cs2StandaloneRadar => {
                &[&Artifact::Cs2RadarClient, &Artifact::DriverInterfaceKernel]
            }
        }
    }

    pub const fn artifact_to_execute(&self) -> &'static Artifact {
        match self {
            Enhancer::Cs2Overlay => &Artifact::Cs2Overlay,
            Enhancer::Cs2StandaloneRadar => &Artifact::Cs2RadarClient,
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum AppCommand {
    /// Quickly launch Valthrun with all the default settings and commands
    QuickStart,

    /// Download and map the driver
    MapDriver,

    /// Download and launch a enhancer
    Launch { enhancer: Enhancer },

    /// Display the version
    Version,
}

async fn real_main(args: AppArgs) -> Result<ExitCode> {
    let http = reqwest::Client::new();

    updater::ui_updater(&http).await?;

    let command = args.command.map(Ok).unwrap_or_else(ui::app_menu)?;

    match command {
        AppCommand::QuickStart => {
            commands::map_driver(&http)
                .await
                .context("execute map driver command")?;

            commands::launch(&http, Enhancer::Cs2Overlay)
                .await
                .context("execute launch enhancer command")?;
        }
        AppCommand::Launch { enhancer } => {
            commands::launch(&http, enhancer)
                .await
                .context("execute launch enhancer command")?;
        }
        AppCommand::MapDriver => {
            commands::map_driver(&http)
                .await
                .context("execute map driver command")?;
        }
        AppCommand::Version => {
            log::info!("Valthrun Loader");
            log::info!("  Version: v{}", env!("CARGO_PKG_VERSION"));
            log::info!("  Build: {} ({})", env!("GIT_HASH"), env!("BUILD_TIME"))
        }
    }

    Ok(ExitCode::SUCCESS)
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = match AppArgs::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Failed to parse arguments:\n{:#}", e);

            if !util::is_console_invoked() {
                util::console_pause();
            }
            return ExitCode::FAILURE;
        }
    };

    env_logger::builder()
        .filter_level(if args.verbose {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Info
        })
        .format_target(args.verbose || cfg!(debug_assertions))
        .parse_default_env()
        .init();

    let status = match real_main(args).await {
        Ok(status) => status,
        Err(e) => {
            log::error!("{:#}", e);
            ExitCode::FAILURE
        }
    };

    if !util::is_console_invoked() {
        util::console_pause();
    }

    status
}
