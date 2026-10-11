#[macro_export]
macro_rules! entry {
    ($kernel_main:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn _start(information: $crate::PhysicalAddress) -> ! {
            $kernel_main(information)
        }
    };
}
