use std::path::{Path, PathBuf};

use crate::platform::Platform;

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
