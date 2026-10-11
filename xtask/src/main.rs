mod cargo;
mod checkpoints;
mod commands;
mod dependencies;
mod esp;
mod firmware;
mod layer;
mod platform;
mod process;
mod qemu;
mod targets;
mod workspace;

use std::process::ExitCode;

use platform::Platform;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(name) = arguments.next() else {
        eprint!("{}", usage());
        return ExitCode::FAILURE;
    };
    let name = name.to_string_lossy();
    let Some(command) = commands::find(&name) else {
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

fn usage() -> String {
    let platforms = Platform::ALL.map(Platform::name).join("|");
    let commands: String = commands::COMMANDS
        .iter()
        .map(|(name, _)| format!("  {name}\n"))
        .collect();
    format!(
        "usage: cargo xtask <command> [--platform {platforms}|all] [options]\ncommands:\n{commands}"
    )
}

#[cfg(test)]
mod tests {
    use super::usage;

    #[test]
    fn usage_offers_every_platform_and_all() {
        assert!(usage().contains("--platform x86_64|aarch64|all"));
    }
}
