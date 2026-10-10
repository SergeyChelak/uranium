use std::path::PathBuf;

use anyhow::Context;
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand};

use crate::layer::Layer;
use crate::platform::Platform;

#[derive(Debug)]
pub struct Declaration {
    pub layer: Layer,
    pub architecture: Option<Platform>,
}

#[derive(Debug)]
pub struct Dependency {
    pub name: String,
    pub development: bool,
}

#[derive(Debug)]
pub struct Package {
    pub name: String,
    pub member: bool,
    pub declaration: Option<Declaration>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub packages: Vec<Package>,
}

impl Workspace {
    pub fn load() -> anyhow::Result<Workspace> {
        let metadata = MetadataCommand::new()
            .exec()
            .context("cargo metadata failed")?;
        Workspace::from_metadata(&metadata)
    }

    pub fn from_metadata(metadata: &Metadata) -> anyhow::Result<Workspace> {
        let packages = metadata
            .packages
            .iter()
            .map(|package| {
                Ok(Package {
                    name: package.name.to_string(),
                    member: metadata.workspace_members.contains(&package.id),
                    declaration: declaration_of(package)?,
                    dependencies: dependencies_of(package),
                })
            })
            .collect::<anyhow::Result<Vec<Package>>>()?;
        Ok(Workspace {
            root: metadata.workspace_root.as_std_path().to_path_buf(),
            packages,
        })
    }

    pub fn package(&self, name: &str) -> Option<&Package> {
        self.packages.iter().find(|package| package.name == name)
    }
}

fn declaration_of(package: &cargo_metadata::Package) -> anyhow::Result<Option<Declaration>> {
    let Some(table) = package.metadata.get("uranium") else {
        return Ok(None);
    };
    let layer_name = table
        .get("layer")
        .and_then(|value| value.as_str())
        .with_context(|| {
            format!(
                "package {} declares no layer under [package.metadata.uranium]",
                package.name
            )
        })?;
    let layer = Layer::parse(layer_name).with_context(|| {
        format!(
            "package {} declares unknown layer {layer_name}",
            package.name
        )
    })?;
    let architecture = match table.get("architecture").and_then(|value| value.as_str()) {
        None => None,
        Some(name) => Some(Platform::parse(name).with_context(|| {
            format!(
                "package {} declares unknown architecture {name}",
                package.name
            )
        })?),
    };
    Ok(Some(Declaration {
        layer,
        architecture,
    }))
}

fn dependencies_of(package: &cargo_metadata::Package) -> Vec<Dependency> {
    package
        .dependencies
        .iter()
        .map(|dependency| Dependency {
            name: dependency.name.clone(),
            development: dependency.kind == DependencyKind::Development,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use cargo_metadata::{Metadata, MetadataCommand};

    use super::Workspace;
    use crate::layer::Layer;
    use crate::platform::Platform;

    struct Spec<'a> {
        name: &'a str,
        member: bool,
        uranium: Option<&'a str>,
        dependencies: &'a [(&'a str, &'a str)],
    }

    const NORMAL: &str = "null";
    const DEVELOPMENT: &str = "\"dev\"";

    fn package_json(spec: &Spec<'_>) -> String {
        let dependencies: Vec<String> = spec
            .dependencies
            .iter()
            .map(|(name, kind)| {
                format!(
                    r#"{{"name":"{name}","req":"*","kind":{kind},"optional":false,"uses_default_features":true,"features":[]}}"#
                )
            })
            .collect();
        let metadata = spec.uranium.map_or("null".to_string(), |table| {
            format!(r#"{{"uranium":{table}}}"#)
        });
        format!(
            r#"{{"name":"{0}","version":"0.1.0","id":"{0} 0.1.0","dependencies":[{1}],"targets":[],"features":{{}},"manifest_path":"/root/{0}/Cargo.toml","metadata":{metadata}}}"#,
            spec.name,
            dependencies.join(",")
        )
    }

    fn metadata(specs: &[Spec<'_>]) -> Metadata {
        let packages: Vec<String> = specs.iter().map(package_json).collect();
        let members: Vec<String> = specs
            .iter()
            .filter(|spec| spec.member)
            .map(|spec| format!(r#""{} 0.1.0""#, spec.name))
            .collect();
        let json = format!(
            r#"{{"packages":[{}],"workspace_members":[{}],"resolve":null,"workspace_root":"/root","target_directory":"/root/target","version":1}}"#,
            packages.join(","),
            members.join(",")
        );
        MetadataCommand::parse(json).unwrap()
    }

    fn workspace(specs: &[Spec<'_>]) -> anyhow::Result<Workspace> {
        Workspace::from_metadata(&metadata(specs))
    }

    #[test]
    fn reads_declared_layer() {
        let workspace = workspace(&[Spec {
            name: "address",
            member: true,
            uranium: Some(r#"{"layer":"lib"}"#),
            dependencies: &[],
        }])
        .unwrap();

        let package = &workspace.packages[0];
        assert_eq!(package.name, "address");
        assert!(package.member);
        assert_eq!(package.declaration.as_ref().unwrap().layer, Layer::Library);
    }

    #[test]
    fn reads_workspace_root() {
        let workspace = workspace(&[]).unwrap();
        assert_eq!(workspace.root, Path::new("/root"));
    }

    #[test]
    fn reads_declared_architecture() {
        let workspace = workspace(&[Spec {
            name: "hal-x86_64",
            member: true,
            uranium: Some(r#"{"layer":"hal/implementation","architecture":"x86_64"}"#),
            dependencies: &[],
        }])
        .unwrap();

        let declaration = workspace.packages[0].declaration.as_ref().unwrap();
        assert_eq!(declaration.layer, Layer::HalImplementation);
        assert_eq!(declaration.architecture, Some(Platform::X86_64));
    }

    #[test]
    fn declaration_without_architecture_is_for_every_platform() {
        let workspace = workspace(&[Spec {
            name: "address",
            member: true,
            uranium: Some(r#"{"layer":"lib"}"#),
            dependencies: &[],
        }])
        .unwrap();

        let declaration = workspace.packages[0].declaration.as_ref().unwrap();
        assert_eq!(declaration.architecture, None);
    }

    #[test]
    fn unknown_architecture_is_an_error() {
        let error = workspace(&[Spec {
            name: "hal-riscv64",
            member: true,
            uranium: Some(r#"{"layer":"hal/implementation","architecture":"riscv64"}"#),
            dependencies: &[],
        }])
        .unwrap_err();

        assert!(error.to_string().contains("hal-riscv64"));
        assert!(error.to_string().contains("riscv64"));
    }

    #[test]
    fn package_without_uranium_table_is_foreign() {
        let workspace = workspace(&[Spec {
            name: "serde",
            member: false,
            uranium: None,
            dependencies: &[],
        }])
        .unwrap();

        let package = &workspace.packages[0];
        assert!(!package.member);
        assert!(package.declaration.is_none());
    }

    #[test]
    fn missing_layer_is_an_error() {
        let error = workspace(&[Spec {
            name: "address",
            member: true,
            uranium: Some("{}"),
            dependencies: &[],
        }])
        .unwrap_err();

        assert!(error.to_string().contains("address"));
        assert!(error.to_string().contains("layer"));
    }

    #[test]
    fn unknown_layer_name_is_an_error() {
        let error = workspace(&[Spec {
            name: "address",
            member: true,
            uranium: Some(r#"{"layer":"kernel/magic"}"#),
            dependencies: &[],
        }])
        .unwrap_err();

        assert!(error.to_string().contains("kernel/magic"));
    }

    #[test]
    fn reads_dependencies_with_their_kind() {
        let workspace = workspace(&[
            Spec {
                name: "address",
                member: true,
                uranium: Some(r#"{"layer":"lib"}"#),
                dependencies: &[("hal", NORMAL), ("serde", DEVELOPMENT)],
            },
            Spec {
                name: "hal",
                member: true,
                uranium: Some(r#"{"layer":"hal/contract"}"#),
                dependencies: &[],
            },
            Spec {
                name: "serde",
                member: false,
                uranium: None,
                dependencies: &[],
            },
        ])
        .unwrap();

        let dependencies = &workspace.packages[0].dependencies;
        assert_eq!(dependencies.len(), 2);
        assert_eq!(dependencies[0].name, "hal");
        assert!(!dependencies[0].development);
        assert_eq!(dependencies[1].name, "serde");
        assert!(dependencies[1].development);
    }

    #[test]
    fn keeps_every_package_of_the_graph() {
        let workspace = workspace(&[
            Spec {
                name: "xtask",
                member: true,
                uranium: Some(r#"{"layer":"tooling"}"#),
                dependencies: &[("anyhow", NORMAL)],
            },
            Spec {
                name: "anyhow",
                member: false,
                uranium: None,
                dependencies: &[],
            },
        ])
        .unwrap();

        let names: Vec<&str> = workspace
            .packages
            .iter()
            .map(|package| package.name.as_str())
            .collect();
        assert_eq!(names, ["xtask", "anyhow"]);
    }
}
