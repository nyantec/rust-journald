pub use std::io::{Error, Result};

use libsystemd_sys as ffi;

/// Convert a systemd ffi return value into a Result
pub fn ffi_result(ret: ffi::c_int) -> Result<ffi::c_int> {
	if ret < 0 {
		Err(Error::from_raw_os_error(-ret))
	} else {
		Ok(ret)
	}
}

/// An analogue of `try!()` for systemd FFI calls.
///
/// The parameter should be a call to a systemd FFI fn with an c_int return
/// value. It is called, and if the return is negative then `sd_try!()`
/// interprets it as an error code and returns IoError from the enclosing fn.
/// Otherwise, the value of `sd_try!()` is the non-negative value returned by
/// the FFI call.
#[deprecated(since = "0.7.0", note = "use `unsafe {ffi_result(expr)}?`")]
#[macro_export]
macro_rules! sd_try {
	($e:expr) => {{
		unsafe { $crate::ffi_result($e) }?
	}};
}

#[path = "journal_entry.rs"]
mod entry;
pub use self::entry::*;

pub mod reader;

#[path = "journal_writer.rs"]
pub mod writer;
