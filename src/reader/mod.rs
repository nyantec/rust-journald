use std::collections::BTreeMap;
use std::ptr;

use libc::{c_char, c_int, free, size_t};
use libsystemd_sys::journal as ffi;

use super::{JournalEntry, Result};
use crate::ffi_result;

mod iter;
#[doc(inline)]
pub use iter::JournalBlockingIter;

// A single log entry from journal.

pub struct JournalReaderConfig {
	/// Set of journald files to read.
	pub files: JournalFiles,

	/// open only volatile journal files, excluding those which are stored on
	/// persistent storage
	pub only_volatile: bool,

	/// open only journal files generated on the local machine
	pub only_local: bool,

	/// read from all namespaces.
	/// only applicable with open_namespace.
	#[cfg(feature = "systemd_v245")]
	pub all_namespaces: bool,

	/// read from the specified namespace and the default namespace.
	/// only applicable with open_namespace.
	#[cfg(feature = "systemd_v245")]
	pub include_default_namespace: bool,
}

impl Default for JournalReaderConfig {
	fn default() -> JournalReaderConfig {
		JournalReaderConfig {
			files: JournalFiles::All,
			only_volatile: false,
			only_local: false,

			#[cfg(feature = "systemd_v245")]
			all_namespaces: false,

			#[cfg(feature = "systemd_v245")]
			include_default_namespace: false,
		}
	}
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
	Cursor(String),
}

/// A reader for systemd journal.
///
/// Supports read, next, previous, and seek operations.
#[repr(transparent)]
pub struct JournalReader {
	j: *mut ffi::sd_journal,
}

impl JournalReader {
	/// Open the systemd journal for reading.
	pub fn open(config: &JournalReaderConfig) -> Result<JournalReader> {
		let mut flags: c_int = 0;

		if config.only_volatile {
			flags |= ffi::SD_JOURNAL_RUNTIME_ONLY;
		}

		if config.only_local {
			flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
		}

		flags |= match config.files {
			JournalFiles::System => ffi::SD_JOURNAL_SYSTEM,
			JournalFiles::CurrentUser => ffi::SD_JOURNAL_CURRENT_USER,
			JournalFiles::All => 0,
		};

		let mut journal = JournalReader { j: ptr::null_mut() };
		unsafe { ffi_result(ffi::sd_journal_open(&mut journal.j, flags))? };

		Ok(journal)
	}

	/// Open the systemd journal for reading from a specific namespace.
	#[cfg(feature = "systemd_v245")]
	pub fn open_namespace(config: &JournalReaderConfig, namespace: &str) -> Result<Self> {
		let mut flags: c_int = 0;

		if config.only_volatile {
			flags |= ffi::SD_JOURNAL_RUNTIME_ONLY;
		}

		if config.only_local {
			flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
		}

		flags |= match config.files {
			JournalFiles::System => ffi::SD_JOURNAL_SYSTEM,
			JournalFiles::CurrentUser => ffi::SD_JOURNAL_CURRENT_USER,
			JournalFiles::All => 0,
		};

		if config.all_namespaces {
			flags |= ffi::SD_JOURNAL_ALL_NAMESPACES;
		}

		if config.include_default_namespace {
			flags |= ffi::SD_JOURNAL_INCLUDE_DEFAULT_NAMESPACE;
		}

		let mut journal = JournalReader { j: ptr::null_mut() };
		unsafe {
			ffi_result(ffi::sd_journal_open_namespace(
				&mut journal.j,
				::std::ffi::CString::new(namespace)?.as_ptr(),
				flags,
			))
		}?;

		Ok(journal)
	}

	/// Get and parse the currently journal entry from the journal
	/// It returns Result<Option<...>> out of convenience for calling
	/// functions. It always returns Ok(Some(...)) if successful.
	fn current_entry(&mut self) -> Result<Option<JournalEntry>> {
		unsafe { ffi::sd_journal_restart_data(self.j) }

		let mut fields = BTreeMap::new();
		let mut sz: size_t = 0;
		let mut data: *const u8 = ptr::null();
		while unsafe {
			ffi_result(ffi::sd_journal_enumerate_data(
				self.j,
				&mut data as *mut *const u8,
				&mut sz,
			))?
		} > 0
		{
			unsafe {
				let b = ::std::slice::from_raw_parts(data, sz as usize);
				let field = String::from_utf8_lossy(b);
				let mut name_value = field.splitn(2, '=');
				let name = name_value.next().unwrap();
				let value = name_value.next().unwrap();
				fields.insert(From::from(name), From::from(value));
			}
		}

		let mut timestamp_realtime_us: u64 = 0;
		unsafe {
			ffi::sd_journal_get_realtime_usec(self.j, &mut timestamp_realtime_us);
		}

		fields.insert(
			"__REALTIME_TIMESTAMP".to_string(),
			timestamp_realtime_us.to_string(),
		);

		let mut timestamp_monotonic_us: u64 = 0;
		unsafe {
			ffi::sd_journal_get_monotonic_usec(
				self.j,
				&mut timestamp_monotonic_us,
				ptr::null_mut(),
			);
		}

		fields.insert(
			"__MONOTONIC_TIMESTAMP".to_string(),
			timestamp_monotonic_us.to_string(),
		);

		let cursor;
		let mut b: *const c_char = ptr::null();
		unsafe {
			ffi::sd_journal_get_cursor(self.j, &mut b);
			cursor = ::std::ffi::CStr::from_ptr(b);
		}

		fields.insert(
			"__CURSOR".to_string(),
			cursor.to_string_lossy().into_owned(),
		);

		unsafe {
			free(b as *mut ::libc::c_void);
		}

		let entry = JournalEntry::from(&fields);

		Ok(Some(entry))
	}

	/// Read the next entry from the journal. Returns `Ok(None)` if there
	/// are no more entrys to read.
	pub fn next_entry(&mut self) -> Result<Option<JournalEntry>> {
		if unsafe { ffi_result(ffi::sd_journal_next(self.j))? } == 0 {
			return Ok(None);
		}

		self.current_entry()
	}

	/// Read the previous entry from the journal. Returns `Ok(None)` if there
	/// are no more entries to read.
	pub fn previous_entry(&mut self) -> Result<Option<JournalEntry>> {
		if unsafe { ffi_result(ffi::sd_journal_previous(self.j))? } == 0 {
			return Ok(None);
		}

		self.current_entry()
	}

	/// Seek to a specific position in journal. On success, returns a cursor
	/// to the current entry.
	pub fn seek(&mut self, seek: JournalSeek) -> Result<()> {
		match seek {
			JournalSeek::Head => unsafe { ffi_result(ffi::sd_journal_seek_head(self.j))? },
			JournalSeek::Tail => unsafe { ffi_result(ffi::sd_journal_seek_tail(self.j))? },
			JournalSeek::Cursor(cur) => unsafe {
				ffi_result(ffi::sd_journal_seek_cursor(
					self.j,
					::std::ffi::CString::new(cur)?.as_ptr(),
				))?
			},
		};

		Ok(())
	}

	/// Sync wait until timeout for new journal messages
	pub fn wait_timeout(&mut self, timeout: u64) -> Result<()> {
		unsafe { ffi_result(ffi::sd_journal_wait(self.j, timeout))? };

		Ok(())
	}

	/// Sync wait forever for new journal messages
	pub fn wait(&mut self) -> Result<()> {
		self.wait_timeout(u64::MAX)
	}

	pub fn add_filter(&mut self, filter: &str) -> Result<()> {
		unsafe {
			ffi_result(ffi::sd_journal_add_match(
				self.j,
				std::ffi::CString::new(filter)?.as_ptr() as *mut std::ffi::c_void,
				0,
			))?
		};

		Ok(())
	}

	/// Create a blocking Iterator from the reader.
	pub fn to_blocking_iter(&mut self) -> JournalBlockingIter {
		JournalBlockingIter { reader: self }
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
