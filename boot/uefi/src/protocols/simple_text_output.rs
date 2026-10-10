use core::mem::offset_of;

use crate::Status;
use crate::firmware_pointer::UnusedFunctionPointer;
use crate::ucs2::{CHUNK_LENGTH, Encoder};

// UEFI 2.11, 12.4.3
type OutputStringFunction =
    unsafe extern "efiapi" fn(*const SimpleTextOutputProtocol, *const u16) -> Status;

// UEFI 2.11, 12.4.1; declared up to OutputString, the last field the loader calls
#[repr(C)]
pub struct SimpleTextOutputProtocol {
    reset: UnusedFunctionPointer,
    output_string: OutputStringFunction,
}

impl SimpleTextOutputProtocol {
    /// Writes the text chunk by chunk and stops at the first error
    pub fn write_str(&self, text: &str) -> Result<(), Status> {
        let mut encoder = Encoder::new(text);
        let mut buffer = [0; CHUNK_LENGTH + 1];
        while encoder.next_chunk(&mut buffer).is_some() {
            // SAFETY: `self` lives in a firmware table that is valid while boot
            // services run, and `buffer` is a null-terminated string that
            // outlives the call
            unsafe { (self.output_string)(self, buffer.as_ptr()) }.into_result()?;
        }
        Ok(())
    }
}

const _: () = assert!(offset_of!(SimpleTextOutputProtocol, output_string) == 8);
