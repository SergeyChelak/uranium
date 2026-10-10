use anyhow::bail;

use crate::cargo;
use crate::esp::Esp;
use crate::firmware;
use crate::platform::{Platform, platforms_from};
use crate::qemu::{command, prepare_variables};
use crate::workspace::Workspace;

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    let platforms = platforms_from(&mut arguments, &[Platform::Aarch64])?;
    let workspace = Workspace::load()?;
    for platform in platforms {
        start(&workspace, platform)?;
    }
    Ok(())
}

/// Boots the platform's ESP directory in QEMU with the serial port on this terminal
pub fn start(workspace: &Workspace, platform: Platform) -> anyhow::Result<()> {
    let esp = Esp::for_platform(&workspace.target_directory, platform);
    if !esp.loader_path().exists() {
        bail!(
            "{} does not exist; run `cargo xtask esp` first",
            esp.loader_path().display()
        );
    }
    let firmware = firmware::find(platform)?;
    let variables = prepare_variables(
        &workspace.target_directory,
        platform,
        &firmware.variables_template,
    )?;
    println!(
        "qemu ({}): firmware {}",
        platform.name(),
        firmware.code.display()
    );
    cargo::run(&mut command(platform, esp.root(), &firmware, &variables))
}
