use crate::{AppCommand, components::Enhancer};

struct Menu {
    title: &'static str,
    items: &'static [MenuItem],
}

struct MenuItem {
    name: &'static str,
    action: MenuAction,
}

enum MenuAction {
    Command(AppCommand),
    Submenu(&'static Menu),
}

const APP_MENU: &Menu = &Menu {
    title: "Please select the command you want to execute:",
    items: &[
        MenuItem {
            name: "Quick launch Valthrun CS2 Overlay with KDMapper",
            action: MenuAction::Command(AppCommand::QuickStart),
        },
        MenuItem {
            name: "Launch an individual component",
            action: MenuAction::Submenu(APP_MENU_LAUNCH),
        },
        MenuItem {
            name: "Show loader version",
            action: MenuAction::Command(AppCommand::Version),
        },
    ],
};

const APP_MENU_LAUNCH: &Menu = &Menu {
    title: "Please select the component you want to launch:",
    items: &[
        MenuItem {
            name: "Map the kernel driver via KDMapper",
            action: MenuAction::Command(AppCommand::MapDriver),
        },
        MenuItem {
            name: "Start the CS2 overlay",
            action: MenuAction::Command(AppCommand::Launch {
                enhancer: Enhancer::Cs2Overlay,
            }),
        },
        MenuItem {
            name: "Start the CS2 standalone radar",
            action: MenuAction::Command(AppCommand::Launch {
                enhancer: Enhancer::Cs2StandaloneRadar,
            }),
        },
    ],
};

pub fn app_menu() -> anyhow::Result<AppCommand> {
    log::info!(
        "Welcome to the Valthrun Loader v{} (#{})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );

    let mut current_menu = APP_MENU;
    loop {
        let choice = inquire::Select::new(
            &format!("{}\n", current_menu.title),
            current_menu
                .items
                .iter()
                .map(|item| item.name)
                .collect::<Vec<_>>(),
        )
        .with_help_message("↑↓ to move, enter to select")
        .without_filtering()
        .raw_prompt()?;

        match &current_menu.items[choice.index].action {
            MenuAction::Command(command) => return Ok(command.clone()),
            MenuAction::Submenu(menu) => current_menu = menu,
        }
    }
}
