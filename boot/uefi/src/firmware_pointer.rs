use core::ffi::c_void;

#[repr(transparent)]
pub(crate) struct FirmwarePointer<T>(*const T);

impl<T> FirmwarePointer<T> {
    /// # Safety
    ///
    /// The table this pointer was read from must still be valid; the firmware
    /// then keeps the pointee valid as well. A null pointer gives `None`
    pub(crate) unsafe fn get(&self) -> Option<&T> {
        // SAFETY: the caller guarantees the table, and with it the pointee, is valid
        unsafe { self.0.as_ref() }
    }
}

/// The pointee stays undeclared until a field is first used
pub(crate) struct Unused {
    _opaque: [u8; 0],
}

pub(crate) type UnusedFirmwarePointer = FirmwarePointer<Unused>;

/// Keeps the offsets of the fields after it right until the function is
/// first called
#[repr(transparent)]
pub(crate) struct UnusedFunctionPointer(*const c_void);

#[cfg(test)]
impl UnusedFunctionPointer {
    pub(crate) const fn null() -> Self {
        Self(core::ptr::null())
    }
}

#[cfg(test)]
impl<T> FirmwarePointer<T> {
    pub(crate) fn to(pointee: &T) -> Self {
        Self(pointee)
    }
}
