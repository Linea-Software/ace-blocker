//! Optional Windows registration with the experimental Ace Guard kernel driver.
//!
//! The protocol deliberately contains no process ID. Ace Guard derives the
//! originating process from the WDF request and retains the corresponding
//! process object. If the standalone prototype driver is not installed, callers
//! log the error and continue with the existing userspace lifecycle.

use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AceGuardRole {
    Engine = 1,
    Watchdog = 2,
}

#[cfg(target_os = "windows")]
mod imp {
    use super::{AceGuardRole, io};
    use std::ffi::c_void;
    use std::fs::OpenOptions;
    use std::os::windows::io::AsRawHandle;
    use std::ptr;

    const FILE_DEVICE_UNKNOWN: u32 = 0x22;
    const FILE_WRITE_ACCESS: u32 = 0x0002;
    const METHOD_BUFFERED: u32 = 0;
    const REGISTER_SELF_FUNCTION: u32 = 0x800;
    const PROTOCOL_VERSION: u32 = 1;
    const DEVICE_PATH: &str = r"\\.\AceGuard";

    const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
        (device_type << 16) | (access << 14) | (function << 2) | method
    }

    const IOCTL_ACEGUARD_REGISTER_SELF: u32 = ctl_code(
        FILE_DEVICE_UNKNOWN,
        REGISTER_SELF_FUNCTION,
        METHOD_BUFFERED,
        FILE_WRITE_ACCESS,
    );

    #[repr(C)]
    struct RegisterSelfInput {
        version: u32,
        role: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn DeviceIoControl(
            device: *mut c_void,
            io_control_code: u32,
            input_buffer: *mut c_void,
            input_buffer_size: u32,
            output_buffer: *mut c_void,
            output_buffer_size: u32,
            bytes_returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
    }

    pub fn register_self(role: AceGuardRole) -> io::Result<()> {
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .open(DEVICE_PATH)?;

        let mut input = RegisterSelfInput {
            version: PROTOCOL_VERSION,
            role: role as u32,
        };
        let mut bytes_returned = 0_u32;

        let success = unsafe {
            // SAFETY: `device` is a live handle for the duration of the call.
            // `input` and `bytes_returned` point to initialized stack objects
            // whose lifetimes cover the synchronous DeviceIoControl operation.
            DeviceIoControl(
                device.as_raw_handle().cast(),
                IOCTL_ACEGUARD_REGISTER_SELF,
                (&mut input as *mut RegisterSelfInput).cast(),
                std::mem::size_of::<RegisterSelfInput>() as u32,
                ptr::null_mut(),
                0,
                &mut bytes_returned,
                ptr::null_mut(),
            )
        };

        if success == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn register_self_ioctl_matches_shared_header() {
            assert_eq!(IOCTL_ACEGUARD_REGISTER_SELF, 0x0022_A000);
        }

        #[test]
        fn registration_input_layout_is_stable() {
            assert_eq!(std::mem::size_of::<RegisterSelfInput>(), 8);
        }
    }
}

#[cfg(target_os = "windows")]
pub use imp::register_self;

#[cfg(not(target_os = "windows"))]
pub fn register_self(_role: AceGuardRole) -> io::Result<()> {
    Ok(())
}
