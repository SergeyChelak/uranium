use std::ffi::OsString;

mod build;
mod check;
mod clippy;
mod dependencies;
mod esp;
mod fmt;
mod host_tests;
mod qemu;
mod run;
mod smoke;
mod test;

pub type Command = fn(lexopt::Parser) -> anyhow::Result<()>;

/// Everything one can type after `cargo xtask`, in the order usage lists it
pub const COMMANDS: &[(&str, Command)] = &[
    ("fmt", fmt::run),
    ("clippy", clippy::run),
    ("dependencies", dependencies::run),
    ("check", check::run),
    ("build", build::run),
    ("esp", esp::run),
    ("qemu", qemu::run),
    ("run", run::run),
    ("host-tests", host_tests::run),
    ("smoke", smoke::run),
    ("test", test::run),
];

/// What a composite command passes to a step that takes no options of its own
fn no_arguments() -> lexopt::Parser {
    lexopt::Parser::from_args(std::iter::empty::<OsString>())
}

pub fn find(name: &str) -> Option<Command> {
    COMMANDS
        .iter()
        .find(|(command_name, _)| *command_name == name)
        .map(|(_, command)| *command)
}

#[cfg(test)]
mod tests {
    use super::find;

    #[test]
    fn unknown_command_is_not_found() {
        assert!(find("frobnicate").is_none());
    }
}
