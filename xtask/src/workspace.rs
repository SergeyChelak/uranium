use std::path::PathBuf;

use anyhow::Context;
use cargo_metadata::camino::Utf8Path;
use cargo_metadata::{Metadata, MetadataCommand};

pub struct Package {
    pub name: String,
    pub directory: PathBuf,
}

pub struct Workspace {
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
            .workspace_packages()
            .into_iter()
            .map(|package| {
                Ok(Package {
                    name: package.name.to_string(),
                    directory: directory_of(package, &metadata.workspace_root)?,
                })
            })
            .collect::<anyhow::Result<Vec<Package>>>()?;
        Ok(Workspace { packages })
    }
}

fn directory_of(package: &cargo_metadata::Package, root: &Utf8Path) -> anyhow::Result<PathBuf> {
    let directory = package
        .manifest_path
        .parent()
        .with_context(|| format!("manifest {} has no parent directory", package.manifest_path))?;
    let relative = directory.strip_prefix(root).with_context(|| {
        format!(
            "package {} lives outside the workspace root {root}",
            package.name
        )
    })?;
    Ok(relative.as_std_path().to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use cargo_metadata::{Metadata, MetadataCommand};

    use super::Workspace;

    fn package_json(name: &str, manifest_path: &str) -> String {
        format!(
            r#"{{"name":"{name}","version":"0.1.0","id":"{name} 0.1.0","dependencies":[],"targets":[],"features":{{}},"manifest_path":"{manifest_path}"}}"#
        )
    }

    fn metadata(packages: &[(&str, &str)], members: &[&str]) -> Metadata {
        let packages: Vec<String> = packages
            .iter()
            .map(|(name, manifest_path)| package_json(name, manifest_path))
            .collect();
        let members: Vec<String> = members
            .iter()
            .map(|name| format!(r#""{name} 0.1.0""#))
            .collect();
        let json = format!(
            r#"{{"packages":[{}],"workspace_members":[{}],"resolve":null,"workspace_root":"/root","target_directory":"/root/target","version":1}}"#,
            packages.join(","),
            members.join(",")
        );
        MetadataCommand::parse(json).unwrap()
    }

    #[test]
    fn reads_workspace_packages_with_their_directories() {
        let metadata = metadata(
            &[
                ("address", "/root/lib/address/Cargo.toml"),
                ("xtask", "/root/xtask/Cargo.toml"),
            ],
            &["address", "xtask"],
        );

        let workspace = Workspace::from_metadata(&metadata).unwrap();

        let names: Vec<&str> = workspace
            .packages
            .iter()
            .map(|package| package.name.as_str())
            .collect();
        assert_eq!(names, ["address", "xtask"]);
        assert_eq!(workspace.packages[0].directory, Path::new("lib/address"));
        assert_eq!(workspace.packages[1].directory, Path::new("xtask"));
    }

    #[test]
    fn skips_packages_that_are_not_workspace_members() {
        let metadata = metadata(
            &[
                ("xtask", "/root/xtask/Cargo.toml"),
                ("anyhow", "/registry/anyhow-1.0.0/Cargo.toml"),
            ],
            &["xtask"],
        );

        let workspace = Workspace::from_metadata(&metadata).unwrap();

        assert_eq!(workspace.packages.len(), 1);
        assert_eq!(workspace.packages[0].name, "xtask");
    }
}
