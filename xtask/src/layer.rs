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
    BootBindings,
    BootImplementation,
    BootRoot,
    UserRuntime,
    UserProgram,
    Tooling,
}

impl Layer {
    pub const ALL: [Layer; 14] = [
        Layer::Library,
        Layer::HalContract,
        Layer::HalImplementation,
        Layer::KernelFoundation,
        Layer::KernelInterface,
        Layer::KernelImplementation,
        Layer::KernelRoot,
        Layer::BootInterface,
        Layer::BootBindings,
        Layer::BootImplementation,
        Layer::BootRoot,
        Layer::UserRuntime,
        Layer::UserProgram,
        Layer::Tooling,
    ];

    pub fn parse(name: &str) -> Option<Layer> {
        Layer::ALL.into_iter().find(|layer| layer.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Layer::Library => "lib",
            Layer::HalContract => "hal/contract",
            Layer::HalImplementation => "hal/implementation",
            Layer::KernelFoundation => "kernel/foundation",
            Layer::KernelInterface => "kernel/interface",
            Layer::KernelImplementation => "kernel/implementation",
            Layer::KernelRoot => "kernel/root",
            Layer::BootInterface => "boot/interface",
            Layer::BootBindings => "boot/bindings",
            Layer::BootImplementation => "boot/implementation",
            Layer::BootRoot => "boot/root",
            Layer::UserRuntime => "user/runtime",
            Layer::UserProgram => "user/program",
            Layer::Tooling => "tooling",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Layer;

    #[test]
    fn every_layer_name_round_trips() {
        for layer in Layer::ALL {
            assert_eq!(Layer::parse(layer.name()), Some(layer));
        }
    }

    #[test]
    fn rejects_unknown_layer_name() {
        assert_eq!(Layer::parse("kernel/magic"), None);
        assert_eq!(Layer::parse("kernel"), None);
    }
}
