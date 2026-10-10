//! Typed views of the raw pointers the firmware writes into its tables.

use core::ffi::c_void;

/// A pointer the firmware wrote into one of its tables.
#[repr(transparent)]
pub(crate) struct FirmwarePointer<T>(*const T);

impl<T> FirmwarePointer<T> {
    /// # Safety
    ///
    /// The table this pointer was read from must still be valid; the firmware
    /// then keeps the pointee valid as well. A null pointer gives `None`.
    pub(crate) unsafe fn get(&self) -> Option<&T> {
        // SAFETY: the caller guarantees the table, and with it the pointee, is valid.
        unsafe { self.0.as_ref() }
    }
}

/// What an unused pointer points to. The loader never reads it, so the
/// pointee stays undeclared until a field is first used.
pub(crate) struct Unused {
    _opaque: [u8; 0],
}

pub(crate) type UnusedFirmwarePointer = FirmwarePointer<Unused>;

/// A table slot whose function the loader does not call yet. It keeps the
/// offsets of the fields after it right; the slot gets its real signature
/// when the function is first called.
#[repr(transparent)]
pub(crate) struct UnusedFunctionPointer(*const c_void);
