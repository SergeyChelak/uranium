use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Library,
    HalContract,
    HalImplementation,
    KernelFoundation,
    KernelInterface,
    KernelImplementation,
    KernelRoot,
    BootInterface,
    UefiBindings,
    BootImplementation,
    BootRoot,
    UserRuntime,
    UserProgram,
    Tooling,
}

const BOOT_INTERFACES: [&str; 4] = ["config", "firmware", "handoff", "sequence"];

pub fn classify(directory: &Path, all_directories: &[PathBuf]) -> anyhow::Result<Layer> {
    let segments: Vec<&str> = directory
        .iter()
        .map(|segment| segment.to_str())
        .collect::<Option<_>>()
        .with_context(|| format!("directory {} is not valid UTF-8", directory.display()))?;
    let layer = match segments.as_slice() {
        ["lib", _] => Layer::Library,
        ["hal", "contract"] => Layer::HalContract,
        ["hal", _] => Layer::HalImplementation,
        ["kernel", "main"] => Layer::KernelRoot,
        ["kernel", "foundation", _] => Layer::KernelFoundation,
        ["kernel", "fs" | "drivers", _] => Layer::KernelImplementation,
        ["kernel", name] => kernel_crate(name, all_directories),
        ["boot", name] if BOOT_INTERFACES.contains(name) => Layer::BootInterface,
        ["boot", "uefi"] => Layer::UefiBindings,
        ["boot", "main"] => Layer::BootRoot,
        ["boot", name] if is_boot_variant(name) => Layer::BootImplementation,
        ["user", "runtime"] => Layer::UserRuntime,
        ["user", _] | ["user", "bin", _] => Layer::UserProgram,
        ["xtask"] => Layer::Tooling,
        _ => bail!(
            "directory {} belongs to no layer of the architecture",
            directory.display()
        ),
    };
    Ok(layer)
}

fn kernel_crate(name: &str, all_directories: &[PathBuf]) -> Layer {
    let has_interface = name
        .match_indices('-')
        .filter_map(|(index, _)| name.get(..index))
        .any(|interface| {
            let subsystem = Path::new("kernel").join(interface);
            let foundation = Path::new("kernel/foundation").join(interface);
            all_directories.contains(&subsystem) || all_directories.contains(&foundation)
        });
    if has_interface {
        Layer::KernelImplementation
    } else {
        Layer::KernelInterface
    }
}

fn is_boot_variant(name: &str) -> bool {
    name.split_once('-')
        .is_some_and(|(interface, _)| BOOT_INTERFACES.contains(&interface))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Layer, classify};

    fn layer_of(directory: &str) -> Layer {
        layer_of_with(directory, &[])
    }

    fn layer_of_with(directory: &str, existing: &[&str]) -> Layer {
        let existing: Vec<PathBuf> = existing.iter().map(PathBuf::from).collect();
        classify(Path::new(directory), &existing).unwrap()
    }

    #[test]
    fn lib_crates_are_libraries() {
        assert_eq!(layer_of("lib/address"), Layer::Library);
    }

    #[test]
    fn hal_contract_is_the_hal_contract() {
        assert_eq!(layer_of("hal/contract"), Layer::HalContract);
    }

    #[test]
    fn hal_architecture_crates_are_hal_implementations() {
        assert_eq!(layer_of("hal/x86_64"), Layer::HalImplementation);
        assert_eq!(layer_of("hal/aarch64"), Layer::HalImplementation);
    }

    #[test]
    fn kernel_foundation_crates_are_the_foundation() {
        assert_eq!(layer_of("kernel/foundation/sync"), Layer::KernelFoundation);
    }

    #[test]
    fn kernel_subsystem_is_an_interface() {
        assert_eq!(layer_of("kernel/scheduler"), Layer::KernelInterface);
    }

    #[test]
    fn kernel_variant_is_an_implementation_when_the_interface_exists() {
        assert_eq!(
            layer_of_with("kernel/scheduler-round-robin", &["kernel/scheduler"]),
            Layer::KernelImplementation
        );
    }

    #[test]
    fn kernel_hyphenated_name_without_interface_is_a_subsystem() {
        assert_eq!(
            layer_of_with("kernel/time-keeping", &["kernel/scheduler"]),
            Layer::KernelInterface
        );
    }

    #[test]
    fn foundation_variant_is_an_implementation() {
        assert_eq!(
            layer_of_with("kernel/frames-buddy", &["kernel/foundation/frames"]),
            Layer::KernelImplementation
        );
    }

    #[test]
    fn file_systems_and_drivers_are_implementations() {
        assert_eq!(layer_of("kernel/fs/fat"), Layer::KernelImplementation);
        assert_eq!(
            layer_of("kernel/drivers/virtio-blk"),
            Layer::KernelImplementation
        );
    }

    #[test]
    fn kernel_main_is_the_kernel_root() {
        assert_eq!(layer_of("kernel/main"), Layer::KernelRoot);
    }

    #[test]
    fn boot_interfaces_are_the_four_named_crates() {
        for name in ["config", "firmware", "handoff", "sequence"] {
            assert_eq!(layer_of(&format!("boot/{name}")), Layer::BootInterface);
        }
    }

    #[test]
    fn boot_uefi_is_the_uefi_bindings() {
        assert_eq!(layer_of("boot/uefi"), Layer::UefiBindings);
    }

    #[test]
    fn boot_variants_are_boot_implementations() {
        assert_eq!(layer_of("boot/firmware-uefi"), Layer::BootImplementation);
        assert_eq!(layer_of("boot/handoff-aarch64"), Layer::BootImplementation);
    }

    #[test]
    fn boot_variant_of_an_unknown_interface_is_an_error() {
        assert!(classify(Path::new("boot/driver-uefi"), &[]).is_err());
    }

    #[test]
    fn boot_main_is_the_boot_root() {
        assert_eq!(layer_of("boot/main"), Layer::BootRoot);
    }

    #[test]
    fn user_runtime_is_the_user_runtime() {
        assert_eq!(layer_of("user/runtime"), Layer::UserRuntime);
    }

    #[test]
    fn user_programs_live_in_user_and_user_bin() {
        assert_eq!(layer_of("user/init"), Layer::UserProgram);
        assert_eq!(layer_of("user/bin/cat"), Layer::UserProgram);
    }

    #[test]
    fn xtask_is_tooling() {
        assert_eq!(layer_of("xtask"), Layer::Tooling);
    }

    #[test]
    fn unknown_directory_is_an_error() {
        let error = classify(Path::new("tools/thing"), &[]).unwrap_err();
        assert!(error.to_string().contains("tools/thing"));
    }
}
