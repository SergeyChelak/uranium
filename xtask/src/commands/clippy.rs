use anyhow::Context;

use crate::cargo;
use crate::platform::{Platform, platforms_from};
use crate::targets::{Target, targets_for};
use crate::workspace::Workspace;

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    let platforms = platforms_from(&mut arguments, &Platform::ALL)?;
    let workspace = Workspace::load()?;
    for package in workspace.packages.iter().filter(|package| package.member) {
        let declaration = package.declaration.as_ref().with_context(|| {
            format!(
                "package {} declares no layer under [package.metadata.uranium]",
                package.name
            )
        })?;
        for target in targets_for(declaration.layer, declaration.architecture, &platforms) {
            let mut command = cargo::cargo(&workspace.root);
            command.args(["clippy", "--package", &package.name]);
            match target {
                Target::Host => command.arg("--all-targets"),
                Target::Triple(triple) => command.args(["--target", triple]),
            };
            command.args(["--", "-D", "warnings"]);
            cargo::run(&mut command)?;
            println!("clippy {} ({target}): ok", package.name);
        }
    }
    Ok(())
}
