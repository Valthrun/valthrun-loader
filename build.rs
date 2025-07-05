/**
 * Taken from https://github.com/Valthrun/Valthrun/blob/46676038ee657905cfd99e0d76ade2439b0cf5d5/controller/build.rs
 */
use std::{io::ErrorKind, path::Path, process::Command};

use chrono::Utc;
use embed_manifest::{manifest::ExecutionLevel, new_manifest};
use winres::WindowsResource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    {
        let git_hash = if Path::new(".git").exists() {
            match { Command::new("git").args(&["rev-parse", "HEAD"]).output() } {
                Ok(output) => String::from_utf8(output.stdout).expect("the git hash to be utf-8"),
                Err(error) => {
                    if error.kind() == ErrorKind::NotFound {
                        panic!(
                            "\n\nBuilding the loader requires git.exe to be installed and available in PATH.\nPlease install https://gitforwindows.org.\n\n"
                        );
                    }

                    return Err(error.into());
                }
            }
        } else {
            "0000000".to_string()
        };

        if git_hash.len() < 7 {
            panic!("Expected the git hash to be at least seven characters long");
        }

        let build_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

        println!("cargo:rustc-env=GIT_HASH={}", &git_hash[0..7]);
        println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    }

    {
        let mut resource = WindowsResource::new();
        resource.set_icon("./resources/app-icon.ico");
        resource.set_manifest(
            &new_manifest("Valthrun Loader")
                .requested_execution_level(ExecutionLevel::RequireAdministrator)
                .to_string(),
        );
        resource.compile()?;
    }

    Ok(())
}
