use crate::cargo;
use crate::workspace::Workspace;

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    if let Some(argument) = arguments.next()? {
        return Err(argument.unexpected().into());
    }
    let workspace = Workspace::load()?;
    cargo::run(cargo::cargo(&workspace.root).args(["fmt", "--all", "--check"]))?;
    println!("fmt: ok");
    Ok(())
}
