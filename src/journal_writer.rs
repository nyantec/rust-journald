use ffi::{array_to_iovecs, journal as ffi};
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
