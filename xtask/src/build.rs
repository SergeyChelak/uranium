use std::path::PathBuf;

use anyhow::{Context, bail};

use crate::cargo::{self, kernel_artifact, loader_artifact};
use crate::platform::{Platform, platforms_from};
use crate::workspace::Workspace;

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    let platforms = platforms_from(&mut arguments, &Platform::ALL)?;
    let workspace = Workspace::load()?;
    for platform in platforms {
        let target = &workspace.target_directory;
        build(
            &workspace,
            "kernel",
            platform.kernel_target(),
            kernel_artifact(target, platform),
        )?;
        build(
            &workspace,
            "loader",
            platform.loader_target(),
            loader_artifact(target, platform),
        )?;
    }
    Ok(())
}

fn build(
    workspace: &Workspace,
    package: &str,
    triple: &str,
    artifact: PathBuf,
) -> anyhow::Result<()> {
    cargo::run(cargo::cargo(&workspace.root).args([
        "build",
        "--package",
        package,
        "--target",
        triple,
    ]))
    .with_context(|| format!("building {package} for {triple}"))?;
    if !artifact.exists() {
        bail!(
            "cargo reported success, but {} does not exist",
            artifact.display()
        );
    }
    println!("build {package} ({triple}): {}", artifact.display());
    Ok(())
}
