use super::{host_tests, no_arguments, smoke};

pub fn run(arguments: lexopt::Parser) -> anyhow::Result<()> {
    host_tests::run(no_arguments())?;
    smoke::run(arguments)
}
