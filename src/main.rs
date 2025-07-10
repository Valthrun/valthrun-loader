use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod api;
mod commands;
mod components;
mod driver;
mod fixes;
mod game;
mod ui;
mod updater;
mod utils;
mod version;

#[derive(Parser, Debug)]
pub struct AppArgs {
    /// Enable verbose logging ($env:RUST_LOG="trace")
    #[clap(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<AppCommand>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum AppCommand {
    /// Quickly launch Valthrun with all the default settings and commands
    QuickStart,

    /// Download and map the driver
    MapDriver,

    /// Download and launch a enhancer
    Launch { enhancer: components::Enhancer },

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

            commands::launch(&http, components::Enhancer::Cs2Overlay)
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = match AppArgs::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Failed to parse arguments:\n{:#}", e);

            if !utils::is_console_invoked() {
                utils::console_pause();
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

    if !utils::is_console_invoked() {
        let _ = inquire::prompt_text("Press enter to exit...");
    }

    status
}
