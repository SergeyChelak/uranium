#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    X86_64,
    Aarch64,
}

impl Platform {
    pub const ALL: [Platform; 2] = [Platform::X86_64, Platform::Aarch64];

    pub fn name(self) -> &'static str {
        match self {
            Platform::X86_64 => "x86_64",
            Platform::Aarch64 => "aarch64",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Platform;

    #[test]
    fn names_are_the_rust_target_architectures() {
        assert_eq!(Platform::X86_64.name(), "x86_64");
        assert_eq!(Platform::Aarch64.name(), "aarch64");
    }

    #[test]
    fn all_lists_x86_64_first() {
        assert_eq!(Platform::ALL, [Platform::X86_64, Platform::Aarch64]);
    }
}
