#![no_std]
#![no_main]
#![expect(
    unsafe_code,
    reason = "the UEFI entry point receives raw pointers from the firmware"
)]

use core::hint::spin_loop;
use core::panic::PanicInfo;

use firmware::{Console, ConsoleError};
use firmware_uefi::UefiFirmware;
use uefi::{Handle, Status, SystemTable};

const GREETING: &str = "uranium loader started";

/// The entry point the firmware calls after loading the image
///
/// # Safety
///
/// Only the firmware calls this, with the handle of this image and a pointer
/// to its system table, which stays valid until boot services end
#[unsafe(no_mangle)]
pub unsafe extern "efiapi" fn efi_main(
    image_handle: Handle,
    system_table: *const SystemTable,
) -> Status {
    // SAFETY: the caller guarantees the system table pointer is valid, and the
    // loader never ends boot services in M0
    let system_table = unsafe { &*system_table };
    let firmware = UefiFirmware::new(image_handle, system_table);
    greet(&firmware);
    firmware.shutdown()
}

fn greet(firmware: &UefiFirmware<'_>) {
    greet_on(firmware.text_console());
    greet_on(firmware.serial_console());
}

/// A console the firmware lacks, or a write that fails, must not stop the shutdown
fn greet_on(console: Result<impl Console, ConsoleError>) {
    if let Ok(mut console) = console {
        let _ = console.write_line(GREETING);
    }
}

#[panic_handler]
fn panic(_information: &PanicInfo) -> ! {
    loop {
        spin_loop();
    }
}
