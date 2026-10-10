use std::fmt;

use crate::layer::Layer;
use crate::platform::Platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Host,
    Triple(&'static str),
}

impl fmt::Display for Target {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Host => formatter.write_str("host"),
            Target::Triple(triple) => formatter.write_str(triple),
        }
    }
}

pub fn targets_for(
    layer: Layer,
    architecture: Option<Platform>,
    selected: &[Platform],
) -> Vec<Target> {
    let platforms = selected
        .iter()
        .copied()
        .filter(|platform| architecture.is_none_or(|own| own == *platform));
    let kernel = |platform: Platform| Target::Triple(platform.kernel_target());
    let loader = |platform: Platform| Target::Triple(platform.loader_target());
    use Layer::*;
    match layer {
        Library | HalContract | BootInterface => std::iter::once(Target::Host)
            .chain(platforms.flat_map(|platform| [kernel(platform), loader(platform)]))
            .collect(),
        HalImplementation => platforms
            .flat_map(|platform| [kernel(platform), loader(platform)])
            .collect(),
        KernelFoundation | KernelInterface | KernelImplementation | KernelRoot | UserRuntime
        | UserProgram => platforms.map(kernel).collect(),
        BootBindings | BootImplementation | BootRoot => platforms.map(loader).collect(),
        Tooling => vec![Target::Host],
    }
}

#[cfg(test)]
mod tests {
    use super::{Target, targets_for};
    use crate::layer::Layer;
    use crate::platform::Platform;

    #[test]
    fn targets_for_library_include_host_and_four_triples() {
        assert_eq!(
            targets_for(Layer::Library, None, &Platform::ALL),
            [
                Target::Host,
                Target::Triple("x86_64-unknown-none"),
                Target::Triple("x86_64-unknown-uefi"),
                Target::Triple("aarch64-unknown-none-softfloat"),
                Target::Triple("aarch64-unknown-uefi"),
            ]
        );
    }

    #[test]
    fn targets_for_hal_x86_64_are_its_own_triples() {
        assert_eq!(
            targets_for(
                Layer::HalImplementation,
                Some(Platform::X86_64),
                &Platform::ALL
            ),
            [
                Target::Triple("x86_64-unknown-none"),
                Target::Triple("x86_64-unknown-uefi"),
            ]
        );
    }

    #[test]
    fn targets_for_kernel_are_none_targets() {
        assert_eq!(
            targets_for(Layer::KernelInterface, None, &Platform::ALL),
            [
                Target::Triple("x86_64-unknown-none"),
                Target::Triple("aarch64-unknown-none-softfloat"),
            ]
        );
    }

    #[test]
    fn targets_for_loader_crates_are_uefi_targets() {
        assert_eq!(
            targets_for(Layer::BootImplementation, None, &Platform::ALL),
            [
                Target::Triple("x86_64-unknown-uefi"),
                Target::Triple("aarch64-unknown-uefi"),
            ]
        );
    }

    #[test]
    fn targets_for_tooling_is_host() {
        assert_eq!(
            targets_for(Layer::Tooling, None, &Platform::ALL),
            [Target::Host]
        );
    }

    #[test]
    fn platform_selection_narrows_triples() {
        assert_eq!(
            targets_for(Layer::KernelRoot, None, &[Platform::Aarch64]),
            [Target::Triple("aarch64-unknown-none-softfloat")]
        );
        assert!(
            targets_for(
                Layer::HalImplementation,
                Some(Platform::X86_64),
                &[Platform::Aarch64]
            )
            .is_empty()
        );
    }

    #[test]
    fn host_prints_as_host_and_a_triple_as_itself() {
        assert_eq!(Target::Host.to_string(), "host");
        assert_eq!(
            Target::Triple("x86_64-unknown-none").to_string(),
            "x86_64-unknown-none"
        );
    }
}
