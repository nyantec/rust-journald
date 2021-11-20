use std::env::var;
pub use std::io::{Error, Result};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicUsize};

#[cfg(feature = "open")]
use dlopen::wrapper::{Container, WrapperApi};
use libc::{c_char, c_int, c_void, size_t};
#[cfg(feature = "open")]
#[macro_use]
extern crate dlopen_derive;

#[cfg(feature = "libsystemd-sys")]
use libsystemd_sys as ffi;

#[path = "journal_entry.rs"]
mod entry;
pub use self::entry::*;

pub mod reader;

#[path = "journal_writer.rs"]
pub mod writer;
/// Convert a systemd ffi return value into a Result
pub fn ffi_result(ret: c_int) -> Result<c_int> {
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

#[cfg(feature = "open")]
pub(crate) static mut SYSTEMD_API: Option<Container<SystemdApi>> = None;
#[cfg(feature = "open")]
static SYSTEMD_API_LOADING: AtomicUsize = AtomicUsize::new(0);

#[derive(WrapperApi)]
#[cfg(feature = "open")]
pub(crate) struct SystemdApi {
	sd_journal_open: unsafe extern "C" fn(sd_journal: *mut *mut c_void, flags: c_int) -> c_int,
	sd_journal_open_namespace: unsafe extern "C" fn(
		sd_journal: *mut *mut c_void,
		name: *const c_char,
		flags: c_int,
	) -> c_int,

	sd_journal_restart_data: unsafe extern "C" fn(sd_journal: *mut c_void),
	sd_journal_enumerate_data: unsafe extern "C" fn(
		sd_journal: *mut c_void,
		data: *mut *const u8,
		length: *mut size_t,
	) -> c_int,
	sd_journal_get_realtime_usec:
		unsafe extern "C" fn(sd_journal: *mut c_void, usec: *mut u64) -> c_int,
	sd_journal_get_monotonic_usec: unsafe extern "C" fn(
		sd_journal: *mut c_void,
		usec: *mut u64,
		boot_id: *mut c_void,
	) -> c_int,

	sd_journal_next: unsafe extern "C" fn(sd_journal: *mut c_void) -> c_int,
	sd_journal_previous: unsafe extern "C" fn(sd_journal: *mut c_void) -> c_int,

	sd_journal_seek_head: unsafe extern "C" fn(sd_journal: *mut c_void) -> c_int,
	sd_journal_seek_tail: unsafe extern "C" fn(sd_journal: *mut c_void) -> c_int,
	sd_journal_seek_cursor:
		unsafe extern "C" fn(sd_journal: *mut c_void, coursor: *const c_char) -> c_int,
	sd_journal_get_cursor:
		unsafe extern "C" fn(sd_journal: *mut c_void, coursor: *mut *const c_char) -> c_int,

	sd_journal_wait: unsafe extern "C" fn(sd_journal: *mut c_void, usec: u64) -> c_int,

	sd_journal_add_match:
		unsafe extern "C" fn(sd_journal: *mut c_void, data: *mut c_void, size: size_t) -> c_int,

	sd_journal_sendv: unsafe extern "C" fn(iv: *const writer::const_iovec, n: c_int) -> c_int,

	sd_journal_close: unsafe extern "C" fn(sd_journal: *mut c_void),
}

#[cfg(feature = "open")]
pub(crate) fn open_systemd<'a>() -> Result<&'a mut Container<SystemdApi>> {
	let loading =
		SYSTEMD_API_LOADING.fetch_update(SeqCst, SeqCst, |x| if x == 0 { Some(1) } else { None });
	match loading {
		Err(1) => return Err(Error::from_raw_os_error(libc::EAGAIN)),
		Err(2) => return Err(Error::from_raw_os_error(libc::ENOSYS)),
		Err(4) => return Ok(unsafe { SYSTEMD_API.as_mut() }.unwrap()),
		Ok(0) => (),
		_ => return Err(Error::from_raw_os_error(libc::ENOTRECOVERABLE)),
	}

	let api: std::result::Result<Container<SystemdApi>, dlopen::Error> =
		unsafe { Container::load("libsystemd.so.0") };
	let api = match api {
		Err(dlopen::Error::OpeningLibraryError(x)) => Err(x),
		Err(dlopen::Error::SymbolGettingError(x)) => Err(x),
		Err(_x) => Err(Error::from_raw_os_error(libc::ENOTRECOVERABLE)),
		Ok(v) => Ok(v),
	};

	if let Err(x) = api {
		SYSTEMD_API_LOADING.store(2, SeqCst);
		return Err(x);
	}

	let mut api = api.unwrap();
	unsafe { SYSTEMD_API = Some(api) };
	SYSTEMD_API_LOADING.store(4, SeqCst);
	Ok(unsafe { SYSTEMD_API.as_mut() }.unwrap())
}

#[cfg(test)]
mod test {
	#[cfg(feature = "open")]
	#[test]
	fn open_api() {
		let api = super::open_systemd().unwrap();
	}
}
