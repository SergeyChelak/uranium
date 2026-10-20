use address::PhysicalAddress;

use crate::layout::layout;

/// `base` is a port number for a 16550 and a physical address for a PL011,
/// so the kind decides what it is
#[repr(C)]
pub(crate) struct RawSerialConsole {
    pub(crate) base: u64,
    pub(crate) kind: u32,
    pub(crate) padding: u32,
}

layout!(RawSerialConsole, 16, { base: 0, kind: 8, padding: 12 });

/// The chip behind the serial port; the discriminant is the code that
/// travels in the wire format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialKind {
    Uart16550 = 1,
    Pl011 = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownSerialKind(pub u32);

impl From<SerialKind> for u32 {
    fn from(kind: SerialKind) -> u32 {
        kind as u32
    }
}

impl TryFrom<u32> for SerialKind {
    type Error = UnknownSerialKind;

    fn try_from(code: u32) -> Result<Self, UnknownSerialKind> {
        match code {
            1 => Ok(Self::Uart16550),
            2 => Ok(Self::Pl011),
            other => Err(UnknownSerialKind(other)),
        }
    }
}

/// The port the loader found for the kernel's early console; a 16550 is
/// reached through the x86 I/O port space, a PL011 through memory
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialConsole {
    Uart16550 { port: u16 },
    Pl011 { base: PhysicalAddress },
}

impl SerialConsole {
    /// `None` for a kind this kernel does not know and for a port number
    /// that does not fit the I/O port space
    pub(crate) fn from_raw(raw: &RawSerialConsole) -> Option<Self> {
        match SerialKind::try_from(raw.kind).ok()? {
            SerialKind::Uart16550 => u16::try_from(raw.base)
                .ok()
                .map(|port| Self::Uart16550 { port }),
            SerialKind::Pl011 => Some(Self::Pl011 {
                base: PhysicalAddress::new(raw.base),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_serial_kind_round_trips_through_u32() {
        for kind in [SerialKind::Uart16550, SerialKind::Pl011] {
            assert_eq!(SerialKind::try_from(u32::from(kind)), Ok(kind));
        }
    }

    #[test]
    fn unknown_serial_kind_names_the_code() {
        for code in [0, 3, u32::MAX] {
            assert_eq!(SerialKind::try_from(code), Err(UnknownSerialKind(code)));
        }
    }
}
