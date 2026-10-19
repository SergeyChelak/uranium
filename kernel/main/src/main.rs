#![no_std]
#![no_main]

use address::PhysicalAddress;

#[cfg(target_arch = "x86_64")]
hal_x86_64::entry!(kernel_main);
#[cfg(target_arch = "aarch64")]
hal_aarch64::entry!(kernel_main);

#[expect(
    unused_variables,
    reason = "the kernel does not read the boot information yet"
)]
fn kernel_main(information: PhysicalAddress) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
