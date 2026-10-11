use super::{clippy, dependencies, fmt, no_arguments};

pub fn run(arguments: lexopt::Parser) -> anyhow::Result<()> {
    fmt::run(no_arguments())?;
    clippy::run(arguments)?;
    dependencies::run(no_arguments())
}
