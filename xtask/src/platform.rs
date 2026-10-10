use anyhow::bail;
use lexopt::ValueExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    X86_64,
    Aarch64,
}

impl Platform {
    pub const ALL: [Platform; 2] = [Platform::X86_64, Platform::Aarch64];

    pub fn parse(name: &str) -> Option<Platform> {
        Platform::ALL
            .into_iter()
            .find(|platform| platform.name() == name)
    }

    pub fn parse_selection(text: &str) -> anyhow::Result<Vec<Platform>> {
        if text == "all" {
            return Ok(Platform::ALL.to_vec());
        }
        match Platform::parse(text) {
            Some(platform) => Ok(vec![platform]),
            None => bail!("unknown platform {text}, expected x86_64, aarch64 or all"),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Platform::X86_64 => "x86_64",
            Platform::Aarch64 => "aarch64",
        }
    }

    pub fn kernel_target(self) -> &'static str {
        match self {
            Platform::X86_64 => "x86_64-unknown-none",
            Platform::Aarch64 => "aarch64-unknown-none-softfloat",
        }
    }

    pub fn loader_target(self) -> &'static str {
        match self {
            Platform::X86_64 => "x86_64-unknown-uefi",
            Platform::Aarch64 => "aarch64-unknown-uefi",
        }
    }

    /// The file name the firmware looks for on a removable medium (UEFI 2.11, 3.5.1.1)
    pub fn loader_file_name(self) -> &'static str {
        match self {
            Platform::X86_64 => "BOOTX64.EFI",
            Platform::Aarch64 => "BOOTAA64.EFI",
        }
    }
}

pub fn platforms_from(
    arguments: &mut lexopt::Parser,
    default: &[Platform],
) -> anyhow::Result<Vec<Platform>> {
    let mut selected = None;
    while let Some(argument) = arguments.next()? {
        match argument {
            lexopt::Arg::Long("platform") => {
                let text = arguments.value()?.string()?;
                selected = Some(Platform::parse_selection(&text)?);
            }
            other => return Err(other.unexpected().into()),
        }
    }
    Ok(selected.unwrap_or_else(|| default.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::{Platform, platforms_from};

    #[test]
    fn names_are_the_rust_target_architectures() {
        assert_eq!(Platform::X86_64.name(), "x86_64");
        assert_eq!(Platform::Aarch64.name(), "aarch64");
    }

    #[test]
    fn all_lists_x86_64_first() {
        assert_eq!(Platform::ALL, [Platform::X86_64, Platform::Aarch64]);
    }

    #[test]
    fn loader_file_names_follow_the_boot_protocol() {
        assert_eq!(Platform::X86_64.loader_file_name(), "BOOTX64.EFI");
        assert_eq!(Platform::Aarch64.loader_file_name(), "BOOTAA64.EFI");
    }

    #[test]
    fn parses_platform_names() {
        assert_eq!(Platform::parse("x86_64"), Some(Platform::X86_64));
        assert_eq!(Platform::parse("aarch64"), Some(Platform::Aarch64));
        assert_eq!(Platform::parse("riscv64"), None);
    }

    #[test]
    fn parses_single_platform() {
        assert_eq!(
            Platform::parse_selection("aarch64").unwrap(),
            [Platform::Aarch64]
        );
    }

    #[test]
    fn parses_all_in_order() {
        assert_eq!(Platform::parse_selection("all").unwrap(), Platform::ALL);
    }

    #[test]
    fn rejects_unknown_platform() {
        let error = Platform::parse_selection("riscv64").unwrap_err();
        assert!(error.to_string().contains("riscv64"));
    }

    #[test]
    fn targets_are_installed_by_the_toolchain_file() {
        let toolchain = include_str!("../../rust-toolchain.toml");
        for platform in Platform::ALL {
            assert!(toolchain.contains(platform.kernel_target()));
            assert!(toolchain.contains(platform.loader_target()));
        }
    }

    #[test]
    fn platform_flag_selects_platforms() {
        let mut arguments = lexopt::Parser::from_args(["--platform", "x86_64"]);
        let selected = platforms_from(&mut arguments, &Platform::ALL).unwrap();
        assert_eq!(selected, [Platform::X86_64]);
    }

    #[test]
    fn missing_platform_flag_gives_the_default() {
        let mut arguments = lexopt::Parser::from_args(Vec::<String>::new());
        let selected = platforms_from(&mut arguments, &[Platform::Aarch64]).unwrap();
        assert_eq!(selected, [Platform::Aarch64]);
    }

    #[test]
    fn unknown_flag_is_rejected() {
        let mut arguments = lexopt::Parser::from_args(["--verbose"]);
        assert!(platforms_from(&mut arguments, &Platform::ALL).is_err());
    }
}
