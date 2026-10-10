use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use serde::Deserialize;

use crate::platform::Platform;

/// A UEFI firmware for QEMU: the flash image with its code and the template
/// of its variable store
#[derive(Debug, PartialEq, Eq)]
pub struct Firmware {
    pub code: PathBuf,
    pub variables_template: PathBuf,
}

/// One firmware descriptor, the format QEMU defines in docs/interop/firmware.json
#[derive(Deserialize)]
struct Descriptor {
    #[serde(rename = "interface-types")]
    interface_types: Vec<String>,
    mapping: Mapping,
    targets: Vec<Target>,
    #[serde(default)]
    features: Vec<String>,
}

#[derive(Deserialize)]
struct Mapping {
    device: String,
    executable: Option<File>,
    #[serde(rename = "nvram-template")]
    nvram_template: Option<File>,
}

#[derive(Deserialize)]
struct File {
    filename: PathBuf,
}

#[derive(Deserialize)]
struct Target {
    architecture: String,
    machines: Vec<String>,
}

pub fn find(platform: Platform) -> anyhow::Result<Firmware> {
    let directories = descriptor_directories(platform)?;
    let descriptors = read_descriptors(&directories)?;
    select(&descriptors, platform).with_context(|| {
        format!(
            "no UEFI firmware descriptor for {} without secure boot in {}",
            platform.name(),
            directories
                .iter()
                .map(|directory| directory.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// The directories QEMU searches for descriptors, most specific first
fn descriptor_directories(platform: Platform) -> anyhow::Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    if let Some(configuration) = user_configuration_directory() {
        directories.push(configuration.join("qemu").join("firmware"));
    }
    directories.push(PathBuf::from("/etc/qemu/firmware"));
    for data_directory in data_directories(platform)? {
        directories.push(data_directory.join("firmware"));
    }
    Ok(directories)
}

fn user_configuration_directory() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

/// Asks QEMU where its data lives instead of assuming an installation layout
fn data_directories(platform: Platform) -> anyhow::Result<Vec<PathBuf>> {
    let program = platform.qemu_system();
    let output = Command::new(program)
        .args(["-L", "help"])
        .output()
        .with_context(|| format!("failed to start `{program} -L help`; is QEMU installed?"))?;
    if !output.status.success() {
        bail!("`{program} -L help` failed with {}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// Reads the descriptors by QEMU's rules: a file name seen in an earlier
/// directory hides the same name in later ones, and an empty file hides the
/// descriptor entirely; the result is ordered by file name
fn read_descriptors(directories: &[PathBuf]) -> anyhow::Result<Vec<(String, Descriptor)>> {
    let mut contents_by_name = BTreeMap::new();
    for directory in directories {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries {
            let path = entry?.path();
            let Some(name) = json_file_name(&path) else {
                continue;
            };
            if contents_by_name.contains_key(&name) {
                continue;
            }
            let contents =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            contents_by_name.insert(name, contents);
        }
    }
    Ok(contents_by_name
        .into_iter()
        .filter(|(_, contents)| !contents.trim().is_empty())
        .filter_map(|(name, contents)| match serde_json::from_str(&contents) {
            Ok(descriptor) => Some((name, descriptor)),
            Err(error) => {
                eprintln!("xtask: ignoring firmware descriptor {name}: {error}");
                None
            }
        })
        .collect())
}

fn json_file_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    name.ends_with(".json").then(|| name.to_owned())
}

/// The first descriptor, by file name, that offers UEFI on flash for the
/// platform's machine and does not demand secure boot
fn select(descriptors: &[(String, Descriptor)], platform: Platform) -> Option<Firmware> {
    descriptors
        .iter()
        .map(|(_, descriptor)| descriptor)
        .filter(|descriptor| suits(descriptor, platform))
        .find_map(|descriptor| {
            Some(Firmware {
                code: descriptor.mapping.executable.as_ref()?.filename.clone(),
                variables_template: descriptor.mapping.nvram_template.as_ref()?.filename.clone(),
            })
        })
}

fn suits(descriptor: &Descriptor, platform: Platform) -> bool {
    descriptor.interface_types.iter().any(|kind| kind == "uefi")
        && descriptor.mapping.device == "flash"
        && !descriptor
            .features
            .iter()
            .any(|feature| feature == "secure-boot" || feature == "requires-smm")
        && descriptor.targets.iter().any(|target| {
            target.architecture == platform.name()
                && target
                    .machines
                    .iter()
                    .any(|pattern| machine_matches(pattern, platform.qemu_machine_family()))
        })
}

/// Descriptor machine patterns end in `*` for a family of versioned machines
fn machine_matches(pattern: &str, family: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => family.starts_with(prefix),
        None => pattern == family,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{Descriptor, Firmware, machine_matches, read_descriptors, select};
    use crate::platform::Platform;

    fn descriptor(architecture: &str, machines: &str, features: &str, code: &str) -> Descriptor {
        let json = format!(
            r#"{{
                "interface-types": ["uefi"],
                "mapping": {{
                    "device": "flash",
                    "executable": {{ "filename": "{code}", "format": "raw" }},
                    "nvram-template": {{ "filename": "{code}.vars", "format": "raw" }}
                }},
                "targets": [{{ "architecture": "{architecture}", "machines": [{machines}] }}],
                "features": [{features}]
            }}"#
        );
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn selects_the_first_matching_descriptor_without_secure_boot() {
        let descriptors = vec![
            (
                "50-secure.json".to_owned(),
                descriptor(
                    "x86_64",
                    r#""pc-q35-*""#,
                    r#""secure-boot", "requires-smm""#,
                    "/secure",
                ),
            ),
            (
                "60-aarch64.json".to_owned(),
                descriptor("aarch64", r#""virt-*""#, "", "/aarch64"),
            ),
            (
                "60-x86_64.json".to_owned(),
                descriptor(
                    "x86_64",
                    r#""pc-i440fx-*", "pc-q35-*""#,
                    r#""acpi-s3""#,
                    "/x86_64",
                ),
            ),
        ];
        assert_eq!(
            select(&descriptors, Platform::X86_64),
            Some(Firmware {
                code: PathBuf::from("/x86_64"),
                variables_template: PathBuf::from("/x86_64.vars"),
            })
        );
        assert_eq!(
            select(&descriptors, Platform::Aarch64),
            Some(Firmware {
                code: PathBuf::from("/aarch64"),
                variables_template: PathBuf::from("/aarch64.vars"),
            })
        );
    }

    #[test]
    fn descriptors_for_other_machines_do_not_match() {
        let descriptors = vec![(
            "60-x86_64.json".to_owned(),
            descriptor("x86_64", r#""pc-i440fx-*""#, "", "/i440fx"),
        )];
        assert_eq!(select(&descriptors, Platform::X86_64), None);
    }

    #[test]
    fn machine_patterns_are_families_or_exact_names() {
        assert!(machine_matches("virt-*", "virt-"));
        assert!(machine_matches("pc-q35-*", "pc-q35-"));
        assert!(!machine_matches("pc-i440fx-*", "pc-q35-"));
        assert!(machine_matches("virt-", "virt-"));
        assert!(!machine_matches("virt", "virt-"));
    }

    #[test]
    fn an_earlier_directory_hides_later_descriptors_and_an_empty_file_disables_one() {
        let root =
            std::env::temp_dir().join(format!("uranium-xtask-firmware-{}", std::process::id()));
        let (first, second) = (root.join("first"), root.join("second"));
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(first.join("10-disabled.json"), "").unwrap();
        fs::write(second.join("10-disabled.json"), sample("/hidden")).unwrap();
        fs::write(first.join("20-first.json"), sample("/first")).unwrap();
        fs::write(second.join("20-first.json"), sample("/second")).unwrap();
        fs::write(second.join("30-only-second.json"), sample("/only-second")).unwrap();
        fs::write(second.join("notes.txt"), "not a descriptor").unwrap();

        let descriptors = read_descriptors(&[first, second]).unwrap();
        let names: Vec<&str> = descriptors.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["20-first.json", "30-only-second.json"]);
        assert_eq!(
            descriptors[0]
                .1
                .mapping
                .executable
                .as_ref()
                .unwrap()
                .filename,
            Path::new("/first")
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn sample(code: &str) -> String {
        format!(
            r#"{{"interface-types":["uefi"],"mapping":{{"device":"flash","executable":{{"filename":"{code}","format":"raw"}},"nvram-template":{{"filename":"{code}.vars","format":"raw"}}}},"targets":[{{"architecture":"aarch64","machines":["virt-*"]}}],"features":[]}}"#
        )
    }
}
