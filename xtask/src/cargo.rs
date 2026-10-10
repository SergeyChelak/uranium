use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};

pub fn cargo(root: &Path) -> Command {
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")));
    command.current_dir(root);
    for name in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"] {
        if env::var_os(name).is_some() {
            eprintln!(
                "xtask: ignoring {name} from the environment so that .cargo/config.toml applies"
            );
        }
        command.env_remove(name);
    }
    command
}

pub fn run(command: &mut Command) -> anyhow::Result<()> {
    let description = describe(command);
    let status = command
        .status()
        .with_context(|| format!("failed to start `{description}`"))?;
    if !status.success() {
        bail!("`{description}` failed with {status}");
    }
    Ok(())
}

fn describe(command: &Command) -> String {
    let program = Path::new(command.get_program())
        .file_name()
        .unwrap_or(command.get_program())
        .to_string_lossy();
    let arguments: Vec<String> = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    format!("{program} {}", arguments.join(" "))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::cargo;

    #[test]
    fn cargo_command_drops_rustflags() {
        let command = cargo(Path::new("."));
        let removed: Vec<&OsStr> = command
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| name)
            .collect();
        assert!(removed.contains(&OsStr::new("RUSTFLAGS")));
        assert!(removed.contains(&OsStr::new("CARGO_ENCODED_RUSTFLAGS")));
    }

    #[test]
    fn cargo_command_runs_in_the_given_directory() {
        let command = cargo(Path::new("/somewhere"));
        assert_eq!(command.get_current_dir(), Some(Path::new("/somewhere")));
    }
}
