use std::fmt;

use anyhow::{Context, bail};

use crate::layer::Layer;
use crate::workspace::Workspace;

#[derive(Debug)]
pub struct Violation {
    pub from: String,
    pub to: String,
    pub reason: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Dependency violation: {} depends on {}, but {}",
            self.from, self.to, self.reason
        )
    }
}

pub fn violations(workspace: &Workspace) -> anyhow::Result<Vec<Violation>> {
    let mut found = Vec::new();
    for package in &workspace.packages {
        let Some(declaration) = &package.declaration else {
            if package.member {
                bail!(
                    "package {} declares no layer under [package.metadata.uranium]",
                    package.name
                );
            }
            continue;
        };
        for dependency in &package.dependencies {
            let target = workspace.package(&dependency.name).with_context(|| {
                format!(
                    "package {} depends on {}, which is not in the dependency graph",
                    package.name, dependency.name
                )
            })?;
            let reason = match &target.declaration {
                None if declaration.layer != Layer::Tooling => {
                    Some("the operating system uses no third-party crates".to_string())
                }
                Some(target_declaration)
                    if !dependency.development
                        && !allowed(declaration.layer, target_declaration.layer) =>
                {
                    Some(reason_for(declaration.layer, target_declaration.layer))
                }
                _ => None,
            };
            if let Some(reason) = reason {
                found.push(Violation {
                    from: package.name.clone(),
                    to: dependency.name.clone(),
                    reason,
                });
            }
        }
    }
    Ok(found)
}

pub fn allowed(from: Layer, to: Layer) -> bool {
    allowed_targets(from).contains(&to)
}

fn allowed_targets(from: Layer) -> &'static [Layer] {
    use Layer::*;
    match from {
        Library => &[Library],
        HalContract => &[Library],
        HalImplementation => &[Library, HalContract],
        KernelFoundation => &[Library, HalContract, KernelFoundation],
        KernelInterface | KernelImplementation => {
            &[Library, HalContract, KernelFoundation, KernelInterface]
        }
        KernelRoot => &[
            Library,
            HalContract,
            HalImplementation,
            KernelFoundation,
            KernelInterface,
            KernelImplementation,
        ],
        BootInterface => &[Library, BootInterface],
        BootBindings => &[Library],
        BootImplementation => &[
            Library,
            HalContract,
            HalImplementation,
            BootInterface,
            BootBindings,
        ],
        BootRoot => &[
            Library,
            HalContract,
            HalImplementation,
            BootInterface,
            BootBindings,
            BootImplementation,
        ],
        UserRuntime => &[Library],
        UserProgram => &[Library, UserRuntime],
        Tooling => &[],
    }
}

fn reason_for(from: Layer, to: Layer) -> String {
    use Layer::*;
    match (from, to) {
        (Tooling, _) => "xtask is a development tool outside the operating system".to_string(),
        (_, KernelRoot | BootRoot) => "nothing depends on a composition root".to_string(),
        (KernelImplementation, KernelImplementation) | (BootImplementation, BootImplementation) => {
            "an implementation never depends on another implementation".to_string()
        }
        _ => format!(
            "a {} crate may depend only on {} crates",
            from.name(),
            listed(allowed_targets(from))
        ),
    }
}

fn listed(layers: &[Layer]) -> String {
    let names: Vec<&str> = layers.iter().map(|layer| layer.name()).collect();
    match names.split_last() {
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::violations;
    use crate::layer::Layer;
    use crate::workspace::{Declaration, Dependency, Package, Workspace};

    fn package(name: &str, layer: &str, dependencies: &[&str]) -> Package {
        Package {
            name: name.to_string(),
            member: true,
            declaration: Some(Declaration {
                layer: Layer::parse(layer).unwrap(),
                architecture: None,
            }),
            dependencies: dependencies
                .iter()
                .map(|name| Dependency {
                    name: name.to_string(),
                    development: false,
                })
                .collect(),
        }
    }

    fn foreign(name: &str) -> Package {
        Package {
            name: name.to_string(),
            member: false,
            declaration: None,
            dependencies: Vec::new(),
        }
    }

    fn with_development(mut package: Package, dependencies: &[&str]) -> Package {
        package
            .dependencies
            .extend(dependencies.iter().map(|name| Dependency {
                name: name.to_string(),
                development: true,
            }));
        package
    }

    fn messages(packages: Vec<Package>) -> Vec<String> {
        violations(&Workspace {
            root: PathBuf::from("/root"),
            target_directory: PathBuf::from("/root/target"),
            packages,
        })
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect()
    }

    #[test]
    fn implementation_may_not_depend_on_implementation() {
        let found = messages(vec![
            package("heap-slab", "kernel/implementation", &[]),
            package("frames-buddy", "kernel/implementation", &["heap-slab"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: frames-buddy depends on heap-slab, but an implementation never depends on another implementation"
            ]
        );
    }

    #[test]
    fn library_may_not_depend_on_hal() {
        let found = messages(vec![
            package("hal", "hal/contract", &[]),
            package("address", "lib", &["hal"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: address depends on hal, but a lib crate may depend only on lib crates"
            ]
        );
    }

    #[test]
    fn hal_contract_knows_no_implementation() {
        let found = messages(vec![
            package("hal-x86_64", "hal/implementation", &[]),
            package("hal", "hal/contract", &["hal-x86_64"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: hal depends on hal-x86_64, but a hal/contract crate may depend only on lib crates"
            ]
        );
    }

    #[test]
    fn boot_interface_never_sees_uefi() {
        let found = messages(vec![
            package("uefi", "boot/bindings", &[]),
            package("boot-sequence", "boot/interface", &["uefi"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: boot-sequence depends on uefi, but a boot/interface crate may depend only on lib and boot/interface crates"
            ]
        );
    }

    #[test]
    fn foundation_never_depends_on_a_subsystem() {
        let found = messages(vec![
            package("trap", "kernel/interface", &[]),
            package("sync", "kernel/foundation", &["trap"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: sync depends on trap, but a kernel/foundation crate may depend only on lib, hal/contract and kernel/foundation crates"
            ]
        );
    }

    #[test]
    fn nothing_depends_on_a_root() {
        let found = messages(vec![
            package("kernel", "kernel/root", &[]),
            package("trap", "kernel/interface", &["kernel"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: trap depends on kernel, but nothing depends on a composition root"
            ]
        );
    }

    #[test]
    fn tooling_depends_on_nothing() {
        let found = messages(vec![
            package("address", "lib", &[]),
            package("xtask", "tooling", &["address"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: xtask depends on address, but xtask is a development tool outside the operating system"
            ]
        );
    }

    #[test]
    fn foreign_dependency_of_tooling_is_allowed() {
        let found = messages(vec![
            foreign("anyhow"),
            package("xtask", "tooling", &["anyhow"]),
        ]);
        assert!(found.is_empty());
    }

    #[test]
    fn foreign_dependency_of_an_os_crate_is_a_violation() {
        let found = messages(vec![
            foreign("serde"),
            package("address", "lib", &["serde"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: address depends on serde, but the operating system uses no third-party crates"
            ]
        );
    }

    #[test]
    fn development_dependency_is_exempt_from_the_layer_rule() {
        let found = messages(vec![
            package("hal", "hal/contract", &[]),
            with_development(package("address", "lib", &[]), &["hal"]),
        ]);
        assert!(found.is_empty());
    }

    #[test]
    fn development_dependency_on_a_foreign_crate_is_a_violation() {
        let found = messages(vec![
            foreign("serde"),
            with_development(package("address", "lib", &[]), &["serde"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: address depends on serde, but the operating system uses no third-party crates"
            ]
        );
    }

    #[test]
    fn member_without_declaration_is_an_error() {
        let mut undeclared = foreign("address");
        undeclared.member = true;
        let error = violations(&Workspace {
            root: PathBuf::from("/root"),
            target_directory: PathBuf::from("/root/target"),
            packages: vec![undeclared],
        })
        .unwrap_err();
        assert!(error.to_string().contains("address"));
    }

    #[test]
    fn unknown_dependency_is_an_error() {
        let error = violations(&Workspace {
            root: PathBuf::from("/root"),
            target_directory: PathBuf::from("/root/target"),
            packages: vec![package("trap", "kernel/interface", &["ghost"])],
        })
        .unwrap_err();
        assert!(error.to_string().contains("ghost"));
    }

    const MODULE_MAP: &[(&str, &str, &[&str])] = &[
        ("address", "lib", &[]),
        ("boot-protocol", "lib", &["address"]),
        ("user-abi", "lib", &[]),
        ("elf", "lib", &["address"]),
        ("hal", "hal/contract", &["address"]),
        (
            "hal-x86_64",
            "hal/implementation",
            &["hal", "address", "boot-protocol"],
        ),
        (
            "hal-aarch64",
            "hal/implementation",
            &["hal", "address", "boot-protocol"],
        ),
        ("sync", "kernel/foundation", &["hal"]),
        ("collections", "kernel/foundation", &[]),
        ("frames", "kernel/foundation", &["address", "sync"]),
        (
            "frames-buddy",
            "kernel/implementation",
            &["frames", "address", "collections"],
        ),
        ("heap", "kernel/foundation", &["frames", "sync"]),
        ("heap-slab", "kernel/implementation", &["heap", "frames"]),
        (
            "virtual-memory",
            "kernel/foundation",
            &["hal", "frames", "address", "sync"],
        ),
        (
            "trap",
            "kernel/interface",
            &["hal", "virtual-memory", "sync", "collections"],
        ),
        (
            "time",
            "kernel/interface",
            &["hal", "trap", "sync", "collections"],
        ),
        (
            "task",
            "kernel/interface",
            &["hal", "virtual-memory", "frames", "sync", "collections"],
        ),
        ("scheduler", "kernel/interface", &["task"]),
        (
            "scheduler-round-robin",
            "kernel/implementation",
            &["scheduler", "task", "collections"],
        ),
        (
            "process",
            "kernel/interface",
            &[
                "task",
                "scheduler",
                "virtual-memory",
                "vfs",
                "ipc",
                "elf",
                "user-abi",
            ],
        ),
        (
            "syscall",
            "kernel/interface",
            &[
                "user-abi",
                "process",
                "vfs",
                "ipc",
                "time",
                "virtual-memory",
            ],
        ),
        (
            "ipc",
            "kernel/interface",
            &["task", "virtual-memory", "user-abi"],
        ),
        ("vfs", "kernel/interface", &["sync", "user-abi", "block"]),
        ("fs-tmpfs", "kernel/implementation", &["vfs"]),
        ("fs-devfs", "kernel/implementation", &["vfs", "device"]),
        ("fs-fat", "kernel/implementation", &["vfs", "block"]),
        ("fs-procfs", "kernel/implementation", &["vfs", "process"]),
        ("block", "kernel/interface", &["sync", "collections"]),
        (
            "device",
            "kernel/interface",
            &["hal", "trap", "virtual-memory", "sync"],
        ),
        ("serial-16550", "kernel/implementation", &["device", "tty"]),
        ("pci", "kernel/interface", &["device", "hal"]),
        ("acpi", "kernel/interface", &["address"]),
        ("device-tree", "kernel/interface", &["address"]),
        (
            "virtio-blk",
            "kernel/implementation",
            &["device", "pci", "block"],
        ),
        (
            "virtio-net",
            "kernel/implementation",
            &["device", "pci", "network"],
        ),
        ("ps2", "kernel/implementation", &["device", "tty"]),
        ("framebuffer", "kernel/implementation", &["device", "tty"]),
        ("tty", "kernel/interface", &["vfs", "ipc", "log", "sync"]),
        ("log", "kernel/interface", &["hal", "sync"]),
        ("network", "kernel/interface", &["device", "ipc", "time"]),
        (
            "kernel",
            "kernel/root",
            &[
                "address",
                "boot-protocol",
                "user-abi",
                "elf",
                "hal",
                "hal-x86_64",
                "hal-aarch64",
                "sync",
                "collections",
                "frames",
                "frames-buddy",
                "heap",
                "heap-slab",
                "virtual-memory",
                "trap",
                "time",
                "task",
                "scheduler",
                "scheduler-round-robin",
                "process",
                "syscall",
                "ipc",
                "vfs",
                "fs-tmpfs",
                "fs-devfs",
                "fs-fat",
                "block",
                "device",
                "serial-16550",
                "pci",
                "acpi",
                "device-tree",
                "virtio-blk",
                "ps2",
                "framebuffer",
                "tty",
                "log",
            ],
        ),
        ("boot-config", "boot/interface", &[]),
        ("firmware", "boot/interface", &["address"]),
        (
            "firmware-uefi",
            "boot/implementation",
            &["firmware", "uefi", "address"],
        ),
        ("uefi", "boot/bindings", &[]),
        ("handoff", "boot/interface", &["address", "boot-protocol"]),
        (
            "handoff-x86_64",
            "boot/implementation",
            &["handoff", "hal-x86_64", "boot-protocol", "address"],
        ),
        (
            "handoff-aarch64",
            "boot/implementation",
            &["handoff", "hal-aarch64", "boot-protocol", "address"],
        ),
        (
            "boot-sequence",
            "boot/interface",
            &[
                "boot-config",
                "firmware",
                "handoff",
                "boot-protocol",
                "elf",
                "address",
            ],
        ),
        (
            "loader",
            "boot/root",
            &[
                "boot-sequence",
                "firmware-uefi",
                "handoff-x86_64",
                "handoff-aarch64",
                "uefi",
                "boot-protocol",
                "address",
            ],
        ),
        ("user-runtime", "user/runtime", &["user-abi"]),
        ("init", "user/program", &["user-runtime"]),
        ("sh", "user/program", &["user-runtime"]),
        ("cat", "user/program", &["user-runtime"]),
        ("xtask", "tooling", &["cargo_metadata", "lexopt", "anyhow"]),
    ];

    #[test]
    fn module_map_graph_passes() {
        let mut packages: Vec<Package> = MODULE_MAP
            .iter()
            .map(|(name, layer, dependencies)| package(name, layer, dependencies))
            .collect();
        packages.extend(["cargo_metadata", "lexopt", "anyhow"].map(foreign));
        assert_eq!(messages(packages), Vec::<String>::new());
    }
}
