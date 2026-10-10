use std::path::PathBuf;

use anyhow::Context;

use crate::layer::{Layer, classify};
use crate::workspace::{Package, Workspace};

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    if let Some(argument) = arguments.next()? {
        return Err(argument.unexpected().into());
    }
    let workspace = Workspace::load()?;
    let layers = layers_of(&workspace)?;
    println!("dependencies: ok, {}", package_count(layers.len()));
    Ok(())
}

fn layers_of(workspace: &Workspace) -> anyhow::Result<Vec<(&Package, Layer)>> {
    let directories: Vec<PathBuf> = workspace
        .packages
        .iter()
        .map(|package| package.directory.clone())
        .collect();
    workspace
        .packages
        .iter()
        .map(|package| {
            let layer = classify(&package.directory, &directories)
                .with_context(|| format!("package {}", package.name))?;
            Ok((package, layer))
        })
        .collect()
}

fn package_count(count: usize) -> String {
    if count == 1 {
        "1 package".to_string()
    } else {
        format!("{count} packages")
    }
}
