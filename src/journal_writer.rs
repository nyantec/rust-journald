use libc::c_int;
use ffi::array_to_iovecs;
use ffi::journal as ffi;

/// Send preformatted fields to systemd.
///
/// This is a relatively low-level operation and probably not suitable unless
/// you need precise control over which fields are sent to systemd.
pub fn send_to_journald(args: &[&str]) -> c_int {
	let iovecs = array_to_iovecs(args);
	unsafe { ffi::sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as c_int) }
}

