use anyhow::bail;

use crate::dependencies::violations;
use crate::workspace::Workspace;

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    if let Some(argument) = arguments.next()? {
        return Err(argument.unexpected().into());
    }
    let workspace = Workspace::load()?;
    let found = violations(&workspace)?;
    for violation in &found {
        eprintln!("{violation}");
    }
    if !found.is_empty() {
        bail!("{}", counted(found.len(), "dependency violation"));
    }
    let members = workspace
        .packages
        .iter()
        .filter(|package| package.member)
        .count();
    println!("dependencies: ok, {}", counted(members, "package"));
    Ok(())
}

fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}
