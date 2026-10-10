mod build;
mod check;
mod clippy;
mod dependencies;
mod esp;
mod fmt;
mod qemu;
mod run;

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
];

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
