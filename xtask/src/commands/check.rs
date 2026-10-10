use std::ffi::OsString;

use super::{clippy, dependencies, fmt};

pub fn run(arguments: lexopt::Parser) -> anyhow::Result<()> {
    fmt::run(no_arguments())?;
    clippy::run(arguments)?;
    dependencies::run(no_arguments())
}

fn no_arguments() -> lexopt::Parser {
    lexopt::Parser::from_args(std::iter::empty::<OsString>())
}
