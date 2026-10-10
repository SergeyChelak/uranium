use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

use crate::cargo::{kernel_artifact, loader_artifact};
use crate::platform::{Platform, platforms_from};
use crate::workspace::Workspace;

/// The EFI system partition as a directory, laid out as `boot_protocol.md` requires
pub struct Esp {
    platform: Platform,
    root: PathBuf,
}

impl Esp {
    pub fn for_platform(target_directory: &Path, platform: Platform) -> Self {
        Esp {
            platform,
            root: target_directory.join("esp").join(platform.name()),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn loader_path(&self) -> PathBuf {
        self.root
            .join("EFI")
            .join("BOOT")
            .join(self.platform.loader_file_name())
    }

    pub fn kernel_path(&self) -> PathBuf {
        self.root.join("URANIUM").join("KERNEL.ELF")
    }
}

pub fn run(mut arguments: lexopt::Parser) -> anyhow::Result<()> {
    let platforms = platforms_from(&mut arguments, &Platform::ALL)?;
    let workspace = Workspace::load()?;
    for platform in platforms {
        let esp = Esp::for_platform(&workspace.target_directory, platform);
        place(
            &loader_artifact(&workspace.target_directory, platform),
            &esp.loader_path(),
        )?;
        place(
            &kernel_artifact(&workspace.target_directory, platform),
            &esp.kernel_path(),
        )?;
        println!("esp ({}): {}", platform.name(), esp.root().display());
    }
    Ok(())
}

fn place(artifact: &Path, destination: &Path) -> anyhow::Result<()> {
    if !artifact.exists() {
        bail!(
            "{} does not exist; run `cargo xtask build` first",
            artifact.display()
        );
    }
    let directory = destination
        .parent()
        .with_context(|| format!("{} has no parent directory", destination.display()))?;
    fs::create_dir_all(directory).with_context(|| format!("creating {}", directory.display()))?;
    fs::copy(artifact, destination).with_context(|| {
        format!(
            "copying {} to {}",
            artifact.display(),
            destination.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Esp;
    use crate::platform::Platform;

    #[test]
    fn esp_paths_follow_the_boot_protocol() {
        let x86_64 = Esp::for_platform(Path::new("/root/target"), Platform::X86_64);
        assert_eq!(x86_64.root(), Path::new("/root/target/esp/x86_64"));
        assert_eq!(
            x86_64.loader_path(),
            Path::new("/root/target/esp/x86_64/EFI/BOOT/BOOTX64.EFI")
        );
        assert_eq!(
            x86_64.kernel_path(),
            Path::new("/root/target/esp/x86_64/URANIUM/KERNEL.ELF")
        );
        let aarch64 = Esp::for_platform(Path::new("/root/target"), Platform::Aarch64);
        assert_eq!(
            aarch64.loader_path(),
            Path::new("/root/target/esp/aarch64/EFI/BOOT/BOOTAA64.EFI")
        );
    }
}
