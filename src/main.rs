use std::{path::PathBuf, process::ExitCode};

use anyhow::{Context, Result};
use clap::{ArgAction, Parser, Subcommand, builder::BoolishValueParser};
use serde::Serialize;

mod api;
mod commands;
mod components;
mod driver;
mod fixes;
mod game;
mod github;
mod metrics;
mod portal;
mod ui;
mod updater;
mod utils;
mod version;

#[derive(Parser, Debug, Serialize)]
pub struct AppArgs {
    /// Enable verbose logging ($env:RUST_LOG="trace")
    #[clap(short, long)]
    verbose: bool,

    /// Disable metrics for this application
    #[clap(long)]
    disable_metrics: bool,

    #[command(subcommand)]
    command: Option<AppCommand>,
}

#[derive(Subcommand, Debug, Clone, Serialize)]
pub enum AppCommand {
    /// Quickly launch Valthrun with all the default settings and commands
    QuickStart { enhancer: components::Enhancer },

    /// Download and map the driver
    MapDriver,

    /// Download and launch a enhancer
    Launch { enhancer: components::Enhancer },

    /// Display the version
    Version,

    /// DO NOT USE:  
    /// This subcommand is invoked by the old executable in an update process.  
    /// It's not intended for manual use!
    #[clap(hide = true)]
    ExecuteUpdate(CommandExecuteUpdate),
}

#[derive(Parser, Debug, Clone, Serialize)]
pub struct CommandExecuteUpdate {
    #[clap(long)]
    pub target_file: PathBuf,

    #[clap(long)]
    pub source_version: String,

    #[clap(long)]
    pub source_hash: String,

    #[clap(long, action = ArgAction::Set, value_parser = BoolishValueParser::new())]
    pub console_invoked: bool,
}

async fn real_main(args: AppArgs) -> Result<ExitCode> {
    let http = reqwest::Client::new();

    log::info!(
        "Welcome to the Valthrun Loader v{} (#{})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );
    metrics::add_record("version-pkg", env!("CARGO_PKG_VERSION"));
    metrics::add_record("version-git", env!("GIT_HASH"));
    metrics::add_record("args", serde_json::to_string(&args)?);

    if !matches!(
        &args.command,
        Some(AppCommand::ExecuteUpdate(_) | AppCommand::Version)
    ) {
        /* only check for updates if we're not the updater itself */
        updater::ui_updater(&http).await?;
    }

    let command = args.command.map(Ok).unwrap_or_else(ui::app_menu)?;

    match command {
        AppCommand::QuickStart { enhancer } => {
            commands::map_driver(&http)
                .await
                .context("execute map driver command")?;

            commands::launch(&http, enhancer)
                .await
                .context("execute launch enhancer command")?;
        }
        AppCommand::MapDriver => {
            commands::map_driver(&http)
                .await
                .context("execute map driver command")?;
        }
        AppCommand::Launch { enhancer } => {
            commands::launch(&http, enhancer)
                .await
                .context("execute launch enhancer command")?;
        }
        AppCommand::Version => {
            log::info!("Valthrun Loader");
            log::info!("  Version: v{}", env!("CARGO_PKG_VERSION"));
            log::info!("  Build: {} (#{})", env!("GIT_HASH"), env!("BUILD_TIME"))
        }
        AppCommand::ExecuteUpdate(command) => {
            if let Err(error) = updater::execute(&command).await {
                /* Update failed. Use the spawned console window to notify the user. */
                log::error!("Failed to update the Valthrun loader: {error}");
                utils::console_pause();

                metrics::add_record("update-error", format!("{error}"));
                metrics::shutdown();
                std::process::exit(1);
            } else {
                /*
                 * Update succeeded.
                 * The updated app should have been started automatically.
                 * Exit the updater.
                 */

                metrics::add_record("update-success", "");
                metrics::shutdown();
                std::process::exit(0);
            }
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

    if !args.disable_metrics {
        if let Err(error) = metrics::init() {
            log::debug!("Metrics failed to initialize: {error}");
        }
    }

    let status = match real_main(args).await {
        Ok(status) => {
            metrics::add_record("app-finished", "");
            status
        }
        Err(e) => {
            log::error!("{:#}", e);
            metrics::add_record("error", format!("{e:#}"));
            ExitCode::FAILURE
        }
    };

    metrics::shutdown();
    if !utils::is_console_invoked() {
        let _ = inquire::prompt_text("Press enter to exit...");
    }

    status
}
