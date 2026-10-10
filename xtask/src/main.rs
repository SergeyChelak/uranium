mod cargo;
mod check;
mod clippy;
mod dependencies;
mod fmt;
mod layer;
mod platform;
mod targets;
mod workspace;

use std::process::ExitCode;

use platform::Platform;

type Command = fn(lexopt::Parser) -> anyhow::Result<()>;

const COMMANDS: &[(&str, Command)] = &[
    ("fmt", fmt::run),
    ("clippy", clippy::run),
    ("dependencies", dependencies::run),
    ("check", check::run),
];

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(name) = arguments.next() else {
        eprint!("{}", usage());
        return ExitCode::FAILURE;
    };
    let name = name.to_string_lossy();
    let Some(command) = find_command(&name) else {
        eprintln!("xtask: unknown command {name}");
        return ExitCode::FAILURE;
    };
    match command(lexopt::Parser::from_args(arguments)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn find_command(name: &str) -> Option<Command> {
    COMMANDS
        .iter()
        .find(|(command_name, _)| *command_name == name)
        .map(|(_, command)| *command)
}

fn usage() -> String {
    let platforms = Platform::ALL.map(Platform::name).join("|");
    let commands: String = COMMANDS
        .iter()
        .map(|(name, _)| format!("  {name}\n"))
        .collect();
    format!(
        "usage: cargo xtask <command> [--platform {platforms}|all] [options]\ncommands:\n{commands}"
    )
}

#[cfg(test)]
mod tests {
    use super::{find_command, usage};

    #[test]
    fn usage_offers_every_platform_and_all() {
        assert!(usage().contains("--platform x86_64|aarch64|all"));
    }

    #[test]
    fn unknown_command_is_not_found() {
        assert!(find_command("frobnicate").is_none());
    }
}
