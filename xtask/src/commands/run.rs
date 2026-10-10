use super::{build, esp, qemu};
use crate::platform::{Platform, platforms_from};
use crate::workspace::Workspace;

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    let platforms = platforms_from(&mut arguments, &[Platform::Aarch64])?;
    let workspace = Workspace::load()?;
    for platform in platforms {
        build::for_platform(&workspace, platform)?;
        esp::populate(&workspace, platform)?;
        qemu::start(&workspace, platform)?;
    }
    Ok(())
}
