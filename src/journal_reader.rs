use libc::{c_int, size_t};
use std::ptr;
use std::collections::BTreeMap;
use ffi::journal as ffi;
use super::Result;

// A single log entry from journal.
pub type JournalRecord = BTreeMap<String, String>;

/// A reader for systemd journal.
///
/// Supports read, next, previous, and seek operations.
pub struct JournalReader {
	j: *mut ffi::sd_journal,
}

/// Represents the set of journal files to read.
#[derive(Clone, Debug)]
pub enum JournalFiles {
	/// The system-wide journal.
	System,
	/// The current user's journal.
	CurrentUser,
	/// Both the system-wide journal and the current user's journal.
	All,
}

/// Seeking position in journal.
pub enum JournalSeek {
	Head,
	Tail,
}

impl JournalReader {

	/// Open the systemd journal for reading.
	///
	/// Params:
	///
	/// * files: the set of journal files to read. If the calling process
	///   doesn't have permission to read the system journal, a call to
	///   `Journal::open` with `System` or `All` will succeed, but system
	///   journal entries won't be included. This behavior is due to systemd.
	/// * runtime_only: if true, include only journal entries from the current
	///   boot. If false, include all entries.
	/// * local_only: if true, include only journal entries originating from
	///   localhost. If false, include all entries.
	pub fn open(
			files: JournalFiles,
			runtime_only: bool,
			local_only: bool) -> Result<JournalReader> {
		let mut flags: c_int = 0;

		if runtime_only {
			flags |= ffi::SD_JOURNAL_RUNTIME_ONLY;
		}

		if local_only {
			flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
		}

		flags |= match files {
			JournalFiles::System => ffi::SD_JOURNAL_SYSTEM,
			JournalFiles::CurrentUser => ffi::SD_JOURNAL_CURRENT_USER,
			JournalFiles::All => 0,
		};

		let mut journal = JournalReader { j: ptr::null_mut() };
		sd_try!(ffi::sd_journal_open(&mut journal.j, flags));

		Ok(journal)
	}

	/// Get and parse the currently journal entry from the journal
	/// It returns Result<Option<...>> out of convenience for calling
	/// functions. It always returns Ok(Some(...)) if successful.
	fn current_entry(&mut self) -> Result<Option<JournalRecord>> {
		unsafe { ffi::sd_journal_restart_data(self.j) }

		let mut ret: JournalRecord = BTreeMap::new();

		let mut sz: size_t = 0;
		let data: *mut u8 = ptr::null_mut();
		while sd_try!(ffi::sd_journal_enumerate_data(self.j, &data, &mut sz)) > 0 {
			unsafe {
				let b = ::std::slice::from_raw_parts_mut(data, sz as usize);
				let field = String::from_utf8_lossy(b);
				let mut name_value = field.splitn(2, '=');
				let name = name_value.next().unwrap();
				let value = name_value.next().unwrap();
				ret.insert(From::from(name), From::from(value));
			}
		}

		Ok(Some(ret))
	}

	/// Read the next entry from the journal. Returns `Ok(None)` if there
	/// are no more entrys to read.
	pub fn next_entry(&mut self) -> Result<Option<JournalRecord>> {
		if sd_try!(ffi::sd_journal_next(self.j)) == 0 {
			return Ok(None);
		}

		return self.current_entry();
	}

	/// Read the previous entry from the journal. Returns `Ok(None)` if there
	/// are no more entrys to read.
	pub fn previous_entry(&mut self) -> Result<Option<JournalRecord>> {
		if sd_try!(ffi::sd_journal_previous(self.j)) == 0 {
			return Ok(None);
		}

		return self.current_entry();
	}

	/// Seek to a specific position in journal. On success, returns a cursor
	/// to the current entry.
	pub fn seek(&mut self, seek: JournalSeek) -> Result<()> {
		match seek {
			JournalSeek::Head => sd_try!(ffi::sd_journal_seek_head(self.j)),
			JournalSeek::Tail => sd_try!(ffi::sd_journal_seek_tail(self.j)),
		};

		return Ok(());
	}

}

impl Drop for JournalReader {

	fn drop(&mut self) {
		if !self.j.is_null() {
			unsafe {
				ffi::sd_journal_close(self.j);
			}
		}
	}

}
