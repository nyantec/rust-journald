use libc::c_int;
use ffi::array_to_iovecs;
use ffi::journal as ffi;
use super::{Result, JournalEntry};

pub fn submit(entry: &JournalEntry) -> Result<()> {
	let mut fields = Vec::<String>::new();

	for (k, v) in entry.get_fields() {
		fields.push(format!("{}={}", k, v));
	}

	let fields_iovec = array_to_iovecs(
			&fields
					.iter()
					.map(|v| v.as_str())
					.collect::<Vec<&str>>());

	sd_try!(ffi::sd_journal_sendv(fields_iovec.as_ptr(), fields_iovec.len() as c_int));

	return Ok(());
}

