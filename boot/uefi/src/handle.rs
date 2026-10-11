use core::ffi::c_void;
use core::ptr::NonNull;

// UEFI 2.11, 2.3.1: EFI_HANDLE is a VOID * naming a collection of interfaces
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Handle(NonNull<c_void>);
