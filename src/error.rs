use windows::core::*;
use windows::Win32::Foundation::*;
use log::error;

pub fn handle_win32_error(operation: &str) -> Result<()> {
    unsafe {
        let error = GetLastError();
        if error != WIN32_ERROR(0) {
            error!("{} failed with error code: {}", operation, error.0);
            return Err(Error::from_win32());
        }
    }
    Ok(())
}

#[macro_export]
macro_rules! win32_try {
    ($expr:expr, $msg:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                error!("{}: {}", $msg, e);
                return Err(e);
            }
        }
    };
}
