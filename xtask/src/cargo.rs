use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

use crate::platform::Platform;

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

pub fn artifact_path(target_directory: &Path, triple: &str, name: &str) -> PathBuf {
    target_directory.join(triple).join("debug").join(name)
}

pub fn kernel_artifact(target_directory: &Path, platform: Platform) -> PathBuf {
    artifact_path(target_directory, platform.kernel_target(), "kernel")
}

pub fn loader_artifact(target_directory: &Path, platform: Platform) -> PathBuf {
    artifact_path(target_directory, platform.loader_target(), "loader.efi")
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

    use super::{artifact_path, cargo, kernel_artifact, loader_artifact};
    use crate::platform::Platform;

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
    fn artifact_path_uses_the_debug_profile() {
        assert_eq!(
            artifact_path(Path::new("/root/target"), "x86_64-unknown-none", "kernel"),
            Path::new("/root/target/x86_64-unknown-none/debug/kernel")
        );
    }

    #[test]
    fn artifacts_are_named_after_their_packages() {
        let target = Path::new("/root/target");
        assert_eq!(
            kernel_artifact(target, Platform::Aarch64),
            Path::new("/root/target/aarch64-unknown-none-softfloat/debug/kernel")
        );
        assert_eq!(
            loader_artifact(target, Platform::X86_64),
            Path::new("/root/target/x86_64-unknown-uefi/debug/loader.efi")
        );
    }

    #[test]
    fn cargo_command_runs_in_the_given_directory() {
        let command = cargo(Path::new("/somewhere"));
        assert_eq!(command.get_current_dir(), Some(Path::new("/somewhere")));
    }
}
