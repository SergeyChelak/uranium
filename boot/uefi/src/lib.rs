#![no_std]
#![expect(
    unsafe_code,
    reason = "calls into the firmware through raw pointers it handed over"
)]

#[cfg(test)]
extern crate std;

mod boot_services;
mod firmware_pointer;
mod guid;
mod handle;
mod protocols;
mod runtime_services;
mod status;
mod system_table;
mod table_header;
mod ucs2;

pub use boot_services::BootServices;
pub use guid::Guid;
pub use handle::Handle;
pub use protocols::{Protocol, SerialIoProtocol, SimpleTextOutputProtocol};
pub use runtime_services::RuntimeServices;
pub use status::Status;
pub use system_table::SystemTable;
pub use table_header::TableHeader;
