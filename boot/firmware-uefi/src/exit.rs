use firmware::{ExitError, MemoryKind, MemoryMapSummary, MemoryRegion};
use uefi::{BootServices, Handle, MemoryDescriptor, MemoryType, Status};

// Twice the two attempts of the FreeBSD and Linux loaders; a firmware that
// keeps changing its map must not spin the loader forever
const EXIT_ATTEMPTS: usize = 4;

enum Attempt {
    Exited(MemoryMapSummary),
    StaleKey,
}

pub(crate) fn exit_boot_services(
    boot_services: &BootServices,
    image: Handle,
    raw: &mut [u8],
    regions: &mut [MemoryRegion],
) -> Result<MemoryMapSummary, ExitError> {
    with_retries(|| attempt(boot_services, image, raw, regions))
}

fn with_retries(
    mut attempt: impl FnMut() -> Result<Attempt, ExitError>,
) -> Result<MemoryMapSummary, ExitError> {
    for _ in 0..EXIT_ATTEMPTS {
        if let Attempt::Exited(summary) = attempt()? {
            return Ok(summary);
        }
    }
    Err(ExitError::MapChangedTooOften)
}

fn attempt(
    boot_services: &BootServices,
    image: Handle,
    raw: &mut [u8],
    regions: &mut [MemoryRegion],
) -> Result<Attempt, ExitError> {
    let map = boot_services
        .get_memory_map(raw)
        .map_err(|_| ExitError::Device)?;
    let region_count = convert_all(map.descriptors(), regions)?;
    let summary = MemoryMapSummary {
        region_count,
        raw_length: map.bytes().len(),
        raw_descriptor_size: map.descriptor_size(),
        raw_descriptor_version: map.descriptor_version(),
    };
    // SAFETY: every console, volume and allocation borrows the `UefiFirmware`
    // the caller consumed, so after `Ok` nothing above the entry point can
    // reach a boot service
    match unsafe { boot_services.exit_boot_services(image, map.key()) } {
        Ok(()) => Ok(Attempt::Exited(summary)),
        Err(Status::INVALID_PARAMETER) => Ok(Attempt::StaleKey),
        Err(_) => Err(ExitError::Device),
    }
}

fn convert_all(
    descriptors: impl Iterator<Item = MemoryDescriptor>,
    regions: &mut [MemoryRegion],
) -> Result<usize, ExitError> {
    let mut slots = regions.iter_mut();
    let mut count = 0;
    for descriptor in descriptors {
        *slots.next().ok_or(ExitError::Device)? = convert(&descriptor);
        count += 1;
    }
    Ok(count)
}

fn convert(descriptor: &MemoryDescriptor) -> MemoryRegion {
    MemoryRegion {
        start: descriptor.physical_start(),
        page_count: descriptor.page_count(),
        kind: kind_of(descriptor.memory_type()),
    }
}

fn kind_of(memory_type: Result<MemoryType, u32>) -> MemoryKind {
    match memory_type {
        Ok(
            MemoryType::LoaderCode
            | MemoryType::LoaderData
            | MemoryType::BootServicesCode
            | MemoryType::BootServicesData
            | MemoryType::Conventional,
        ) => MemoryKind::Usable,
        Ok(MemoryType::RuntimeServicesCode | MemoryType::RuntimeServicesData) => {
            MemoryKind::UefiRuntime
        }
        Ok(MemoryType::AcpiReclaim) => MemoryKind::AcpiReclaimable,
        Ok(MemoryType::AcpiNonVolatileStorage) => MemoryKind::AcpiNonVolatile,
        Ok(MemoryType::MemoryMappedIo | MemoryType::MemoryMappedIoPortSpace) => {
            MemoryKind::MemoryMappedIo
        }
        Ok(
            MemoryType::Reserved
            | MemoryType::Unusable
            | MemoryType::PalCode
            | MemoryType::Persistent
            | MemoryType::Unaccepted,
        )
        | Err(_) => MemoryKind::Reserved,
    }
}

#[cfg(test)]
mod tests {
    use super::{Attempt, EXIT_ATTEMPTS, kind_of, with_retries};
    use firmware::{ExitError, MemoryKind, MemoryMapSummary};
    use uefi::MemoryType;

    const SUMMARY: MemoryMapSummary = MemoryMapSummary {
        region_count: 3,
        raw_length: 144,
        raw_descriptor_size: 48,
        raw_descriptor_version: 1,
    };

    #[test]
    fn convert_maps_every_firmware_type() {
        let expected = [
            (MemoryType::Reserved, MemoryKind::Reserved),
            (MemoryType::LoaderCode, MemoryKind::Usable),
            (MemoryType::LoaderData, MemoryKind::Usable),
            (MemoryType::BootServicesCode, MemoryKind::Usable),
            (MemoryType::BootServicesData, MemoryKind::Usable),
            (MemoryType::RuntimeServicesCode, MemoryKind::UefiRuntime),
            (MemoryType::RuntimeServicesData, MemoryKind::UefiRuntime),
            (MemoryType::Conventional, MemoryKind::Usable),
            (MemoryType::Unusable, MemoryKind::Reserved),
            (MemoryType::AcpiReclaim, MemoryKind::AcpiReclaimable),
            (
                MemoryType::AcpiNonVolatileStorage,
                MemoryKind::AcpiNonVolatile,
            ),
            (MemoryType::MemoryMappedIo, MemoryKind::MemoryMappedIo),
            (
                MemoryType::MemoryMappedIoPortSpace,
                MemoryKind::MemoryMappedIo,
            ),
            (MemoryType::PalCode, MemoryKind::Reserved),
            (MemoryType::Persistent, MemoryKind::Reserved),
            (MemoryType::Unaccepted, MemoryKind::Reserved),
        ];
        for (memory_type, kind) in expected {
            assert_eq!(kind_of(Ok(memory_type)), kind, "{memory_type:?}");
        }
    }

    #[test]
    fn an_unknown_type_is_reserved() {
        assert_eq!(kind_of(Err(16)), MemoryKind::Reserved);
        assert_eq!(kind_of(Err(0x8000_0000)), MemoryKind::Reserved);
    }

    #[test]
    fn exit_retries_after_a_stale_key() {
        let mut calls = 0;
        let result = with_retries(|| {
            calls += 1;
            Ok(if calls == 1 {
                Attempt::StaleKey
            } else {
                Attempt::Exited(SUMMARY)
            })
        });
        assert_eq!(result, Ok(SUMMARY));
        assert_eq!(calls, 2);
    }

    #[test]
    fn exit_gives_up_after_the_limit() {
        let mut calls = 0;
        let result = with_retries(|| {
            calls += 1;
            Ok(Attempt::StaleKey)
        });
        assert_eq!(result, Err(ExitError::MapChangedTooOften));
        assert_eq!(calls, EXIT_ATTEMPTS);
    }

    #[test]
    fn exit_stops_at_a_device_error() {
        let mut calls = 0;
        let result = with_retries(|| {
            calls += 1;
            Err(ExitError::Device)
        });
        assert_eq!(result, Err(ExitError::Device));
        assert_eq!(calls, 1);
    }
}
