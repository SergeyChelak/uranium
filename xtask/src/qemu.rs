use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

use crate::firmware::Firmware;
use crate::platform::Platform;

/// The firmware writes its variables into the second flash image, so each
/// platform gets its own copy of the template, made once and kept between runs
pub fn prepare_variables(
    target_directory: &Path,
    platform: Platform,
    template: &Path,
) -> anyhow::Result<PathBuf> {
    let directory = target_directory.join("qemu");
    fs::create_dir_all(&directory).with_context(|| format!("creating {}", directory.display()))?;
    let variables = directory.join(format!("{}-variables.fd", platform.name()));
    if !variables.exists() {
        fs::copy(template, &variables).with_context(|| {
            format!("copying {} to {}", template.display(), variables.display())
        })?;
    }
    Ok(variables)
}

pub fn command(platform: Platform, esp: &Path, firmware: &Firmware, variables: &Path) -> Command {
    let mut command = Command::new(platform.qemu_system());
    match platform {
        Platform::X86_64 => command.args(["-machine", "q35", "-accel", "tcg", "-cpu", "max"]),
        Platform::Aarch64 => command.args([
            "-machine",
            "virt,gic-version=3",
            "-accel",
            "hvf",
            "-cpu",
            "host",
        ]),
    };
    command.args(["-m", "512M"]);
    command.arg("-drive").arg(format!(
        "if=pflash,unit=0,format=raw,readonly=on,file={}",
        firmware.code.display()
    ));
    command.arg("-drive").arg(format!(
        "if=pflash,unit=1,format=raw,file={}",
        variables.display()
    ));
    command
        .arg("-drive")
        .arg(format!("format=raw,file=fat:rw:{}", esp.display()));
    command.args(["-serial", "stdio", "-display", "none", "-no-reboot"]);
    command
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{command, prepare_variables};
    use crate::firmware::Firmware;
    use crate::platform::Platform;

    fn arguments(platform: Platform) -> Vec<String> {
        let firmware = Firmware {
            code: PathBuf::from("/firmware/code.fd"),
            variables_template: PathBuf::from("/firmware/vars.fd"),
        };
        command(
            platform,
            Path::new("/target/esp/x"),
            &firmware,
            Path::new("/target/qemu/x-variables.fd"),
        )
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
    }

    #[test]
    fn aarch64_command_uses_virt_gicv3_and_hvf() {
        let arguments = arguments(Platform::Aarch64);
        assert_eq!(
            command(
                Platform::Aarch64,
                Path::new("/"),
                &Firmware {
                    code: PathBuf::new(),
                    variables_template: PathBuf::new()
                },
                Path::new("/")
            )
            .get_program(),
            "qemu-system-aarch64"
        );
        assert!(arguments.contains(&"virt,gic-version=3".to_owned()));
        assert!(arguments.contains(&"hvf".to_owned()));
        assert!(arguments.contains(&"format=raw,file=fat:rw:/target/esp/x".to_owned()));
        assert!(arguments.contains(
            &"if=pflash,unit=0,format=raw,readonly=on,file=/firmware/code.fd".to_owned()
        ));
        assert!(
            arguments.contains(
                &"if=pflash,unit=1,format=raw,file=/target/qemu/x-variables.fd".to_owned()
            )
        );
    }

    #[test]
    fn x86_64_command_uses_q35_and_tcg() {
        let arguments = arguments(Platform::X86_64);
        assert!(arguments.contains(&"q35".to_owned()));
        assert!(arguments.contains(&"tcg".to_owned()));
        assert!(arguments.contains(&"-no-reboot".to_owned()));
    }

    #[test]
    fn variables_are_copied_once() {
        let root = std::env::temp_dir().join(format!("uranium-xtask-qemu-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let template = root.join("template.fd");
        fs::write(&template, b"template").unwrap();

        let variables = prepare_variables(&root, Platform::Aarch64, &template).unwrap();
        assert_eq!(variables, root.join("qemu").join("aarch64-variables.fd"));
        assert_eq!(fs::read(&variables).unwrap(), b"template");

        fs::write(&variables, b"written by the firmware").unwrap();
        prepare_variables(&root, Platform::Aarch64, &template).unwrap();
        assert_eq!(fs::read(&variables).unwrap(), b"written by the firmware");
        fs::remove_dir_all(root).unwrap();
    }
}
