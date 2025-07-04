use std::sync::LazyLock;

use crate::{AppCommand, Enhancer};

const MENU_OPTIONS: LazyLock<Vec<(&'static str, AppCommand)>> = LazyLock::new(|| {
    Vec::from([
        (
            "Launch Valthrun with default settings",
            AppCommand::QuickStart,
        ),
        ("Map Driver", AppCommand::MapDriver),
        (
            "Launch Overlay",
            AppCommand::Launch {
                enhancer: Enhancer::Cs2Overlay,
            },
        ),
        (
            "Launch Standalone Radar",
            AppCommand::Launch {
                enhancer: Enhancer::Cs2StandaloneRadar,
            },
        ),
        ("Show Version", AppCommand::Version),
    ])
});

pub fn app_menu() -> anyhow::Result<AppCommand> {
    log::info!(
        "Welcome to the Valthrun Loader v{} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );

    let choice = inquire::Select::new(
        "Please select the command you want to execute:\n",
        MENU_OPTIONS
            .iter()
            .map(|(name, _value)| *name)
            .collect::<Vec<_>>(),
    )
    .with_help_message("↑↓ to move, enter to select")
    .without_filtering()
    .raw_prompt()?;

    Ok(MENU_OPTIONS[choice.index].1.clone())
}
