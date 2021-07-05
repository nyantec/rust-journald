use ffi::{c_void, const_iovec, journal as ffi, size_t};
use ffi_result;
use libc::c_int;

use super::{JournalEntry, Result};

pub fn submit(entry: &JournalEntry) -> Result<()> {
	let mut fields = Vec::<String>::new();

	for (k, v) in entry.get_fields() {
		fields.push(format!("{}={}", k, v));
	}

	let fields_iovec = array_to_iovecs(&fields.iter().map(|v| v.as_str()).collect::<Vec<&str>>());

	unsafe {
		ffi_result(ffi::sd_journal_sendv(
			fields_iovec.as_ptr(),
			fields_iovec.len() as c_int,
		))?
	};

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
