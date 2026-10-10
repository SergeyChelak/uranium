mod serial_io;
mod simple_text_output;

use crate::Guid;

pub use serial_io::SerialIoProtocol;
pub use simple_text_output::SimpleTextOutputProtocol;

/// A protocol structure the firmware's protocol database knows by `GUID`.
///
/// # Safety
///
/// `GUID` must identify the protocol whose structure the implementing
/// `#[repr(C)]` type declares: `BootServices::locate_protocol` reinterprets
/// the pointer the firmware returns for `GUID` as that type.
pub unsafe trait Protocol {
    const GUID: Guid;
}
