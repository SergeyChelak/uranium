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
        BootImplementation => &[Library, HalImplementation, BootInterface, BootBindings],
        BootRoot => &[
            Library,
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
    fn loader_implementation_never_calls_the_hal_contract() {
        let found = messages(vec![
            package("hal", "hal/contract", &[]),
            package("handoff-x86_64", "boot/implementation", &["hal"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: handoff-x86_64 depends on hal, but a boot/implementation crate may depend only on lib, hal/implementation, boot/interface and boot/bindings crates"
            ]
        );
    }

    #[test]
    fn loader_root_never_calls_the_hal_contract() {
        let found = messages(vec![
            package("hal", "hal/contract", &[]),
            package("loader", "boot/root", &["hal"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: loader depends on hal, but a boot/root crate may depend only on lib, hal/implementation, boot/interface, boot/bindings and boot/implementation crates"
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

    #[test]
    fn interface_never_depends_on_an_implementation() {
        let found = messages(vec![
            package("frames-buddy", "kernel/implementation", &[]),
            package("process", "kernel/interface", &["frames-buddy"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: process depends on frames-buddy, but a kernel/interface crate may depend only on lib, hal/contract, kernel/foundation and kernel/interface crates"
            ]
        );
    }

    #[test]
    fn hal_implementation_knows_no_kernel() {
        let found = messages(vec![
            package("sync", "kernel/foundation", &[]),
            package("hal-x86_64", "hal/implementation", &["sync"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: hal-x86_64 depends on sync, but a hal/implementation crate may depend only on lib and hal/contract crates"
            ]
        );
    }

    #[test]
    fn kernel_root_names_the_implementations() {
        let found = messages(vec![
            package("frames-buddy", "kernel/implementation", &[]),
            package("hal-x86_64", "hal/implementation", &[]),
            package("kernel", "kernel/root", &["frames-buddy", "hal-x86_64"]),
        ]);
        assert!(found.is_empty());
    }

    #[test]
    fn handoff_builds_on_the_hal_implementation() {
        let found = messages(vec![
            package("hal-x86_64", "hal/implementation", &[]),
            package("handoff-x86_64", "boot/implementation", &["hal-x86_64"]),
        ]);
        assert!(found.is_empty());
    }

    #[test]
    fn user_program_sees_only_its_runtime() {
        let found = messages(vec![
            package("user-runtime", "user/runtime", &[]),
            package("syscall", "kernel/interface", &[]),
            package("init", "user/program", &["user-runtime", "syscall"]),
        ]);
        assert_eq!(
            found,
            [
                "Dependency violation: init depends on syscall, but a user/program crate may depend only on lib and user/runtime crates"
            ]
        );
    }
}
