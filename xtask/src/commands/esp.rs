use std::fs;
use std::path::Path;

use anyhow::{Context, bail};

use crate::cargo::{kernel_artifact, loader_artifact};
use crate::esp::Esp;
use crate::platform::{Platform, platforms_from};
use crate::workspace::Workspace;

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    let platforms = platforms_from(&mut arguments, &Platform::ALL)?;
    let workspace = Workspace::load()?;
    for platform in platforms {
        populate(&workspace, platform)?;
    }
    Ok(())
}

/// Copies the built loader and kernel of one platform into its ESP directory
pub fn populate(workspace: &Workspace, platform: Platform) -> anyhow::Result<()> {
    let esp = Esp::for_platform(&workspace.target_directory, platform);
    place(
        &loader_artifact(&workspace.target_directory, platform),
        &esp.loader_path(),
    )?;
    place(
        &kernel_artifact(&workspace.target_directory, platform),
        &esp.kernel_path(),
    )?;
    println!("esp ({}): {}", platform.name(), esp.root().display());
    Ok(())
}

fn place(artifact: &Path, destination: &Path) -> anyhow::Result<()> {
    if !artifact.exists() {
        bail!(
            "{} does not exist; run `cargo xtask build` first",
            artifact.display()
        );
    }
    let directory = destination
        .parent()
        .with_context(|| format!("{} has no parent directory", destination.display()))?;
    fs::create_dir_all(directory).with_context(|| format!("creating {}", directory.display()))?;
    fs::copy(artifact, destination).with_context(|| {
        format!(
            "copying {} to {}",
            artifact.display(),
            destination.display()
        )
    })?;
    Ok(())
}
