use anyhow::{Context, bail};

use crate::cargo::{self, artifact_path};
use crate::platform::{Platform, platforms_from};
use crate::workspace::Workspace;

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    let platforms = platforms_from(&mut arguments, &Platform::ALL)?;
    let workspace = Workspace::load()?;
    for platform in platforms {
        let triple = platform.kernel_target();
        cargo::run(cargo::cargo(&workspace.root).args([
            "build",
            "--package",
            "kernel",
            "--target",
            triple,
        ]))
        .with_context(|| format!("building the kernel for {triple}"))?;
        let artifact = artifact_path(&workspace.target_directory, triple, "kernel");
        if !artifact.exists() {
            bail!(
                "cargo reported success, but {} does not exist",
                artifact.display()
            );
        }
        println!("build kernel ({triple}): {}", artifact.display());
    }
    Ok(())
}
