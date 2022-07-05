use libc::c_int;
use libsystemd_sys::{c_void, const_iovec, journal as ffi, size_t};

use super::{JournalEntry, Result};
use crate::ffi_result;

pub fn submit(entry: &JournalEntry) -> Result<()> {
	let mut fields = Vec::<Vec<u8>>::new();

	for (k, v) in entry.get_fields() {
		let mut field_content = k.as_bytes().to_vec();
		field_content.push(b'=');
		field_content.extend_from_slice(&v);
		fields.push(field_content);
	}

	let fields_iovec =
		array_to_iovecs(&fields.iter().map(|v| v.as_slice()).collect::<Vec<&[u8]>>());

	unsafe {
		ffi_result(ffi::sd_journal_sendv(
			fields_iovec.as_ptr(),
			fields_iovec.len() as c_int,
		))?
	};

	Ok(())
}

pub fn array_to_iovecs(args: &[&[u8]]) -> Vec<const_iovec> {
	args.iter()
		.map(|d| const_iovec {
			iov_base: d.as_ptr() as *const c_void,
			iov_len: d.len() as size_t,
		})
		.collect()
}
