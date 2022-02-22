use libc::{c_int, c_void, size_t};
#[cfg(feature = "libsystemd-sys")]
use libsystemd_sys::{const_iovec, journal as ffi};

use super::{JournalEntry, Result};
use crate::ffi_result;

pub fn submit(entry: &JournalEntry) -> Result<()> {
	let mut fields = Vec::<String>::new();

	for (k, v) in entry.get_fields() {
		fields.push(format!("{}={}", k, v));
	}

	let fields_iovec = array_to_iovecs(&fields.iter().map(|v| v.as_str()).collect::<Vec<&str>>());

	ffi_result(unsafe {
		#[cfg(feature = "libsystemd-sys")]
		{
			ffi::sd_journal_sendv(fields_iovec.as_ptr(), fields_iovec.len() as c_int)
		}

		#[cfg(feature = "open")]
		{
			crate::open_systemd()?
				.sd_journal_sendv(fields_iovec.as_ptr(), fields_iovec.len() as c_int)
		}
	})?;

	Ok(())
}

pub fn array_to_iovecs(args: &[&str]) -> Vec<const_iovec> {
	args.iter()
		.map(|d| const_iovec {
			iov_base: d.as_ptr() as *const c_void,
			iov_len: d.len() as size_t,
		})
		.collect()
}

// START COPY FROM LIBSYSTEMD_SYS
/// Helper type to mark functions systemd functions that promise not to modify the underying iovec
/// data.  There is no corresponding type in libc, so their function signatures take *const iovec,
/// which technically allow iov_base to be modified.  However, const_iovec provides the same ABI, so
/// it can be used to make the function interface easier to work with.
#[cfg(feature = "open")]
#[repr(C)]
pub struct const_iovec {
	pub iov_base: *const c_void,
	pub iov_len: size_t,
}

#[cfg(feature = "open")]
impl const_iovec {
	///
	/// # Safety
	///
	/// Lifetime of `arg` must be long enough to cover future dereferences of the internal
	/// `Self::iov_base` pointer.
	pub unsafe fn from_str<T>(arg: T) -> Self
	where
		T: AsRef<str>,
	{
		const_iovec {
			iov_base: arg.as_ref().as_ptr() as *const c_void,
			iov_len: arg.as_ref().len() as size_t,
		}
	}
}
// END COPY FROM LIBSYSTEMD_SYS
