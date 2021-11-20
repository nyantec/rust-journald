use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::io::Error;
use std::ptr;
use std::time::Duration;

#[cfg(feature = "open")]
use libc::c_void;
use libc::{c_char, c_int, free, size_t};
#[cfg(feature = "libsystemd-sys")]
use libsystemd_sys::journal as ffi;

use super::{JournalEntry, Result};
use crate::ffi_result;

mod iter;
#[doc(inline)]
pub use iter::{JournalBlockingIter, JournalIter};

// A single log entry from journal.

#[allow(dead_code)]
const SD_JOURNAL_LOCAL_ONLY: i32 = 1 << 0;
#[allow(dead_code)]
const SD_JOURNAL_RUNTIME_ONLY: i32 = 1 << 1;
#[allow(dead_code)]
const SD_JOURNAL_SYSTEM: i32 = 1 << 2;
#[allow(dead_code)]
const SD_JOURNAL_CURRENT_USER: i32 = 1 << 3;
#[allow(dead_code)]
const SD_JOURNAL_OS_ROOT: i32 = 1 << 4;
#[allow(dead_code)]

/// Show all namespaces, not just the default or specified one
#[allow(dead_code)]
const SD_JOURNAL_ALL_NAMESPACES: i32 = 1 << 5;

/// Show default namespace in addition to specified one
#[allow(dead_code)]
const SD_JOURNAL_INCLUDE_DEFAULT_NAMESPACE: i32 = 1 << 6;

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
	#[cfg(any(feature = "systemd_v245", feature = "dl_namespace"))]
	pub all_namespaces: bool,

	/// read from the specified namespace and the default namespace.
	/// only applicable with open_namespace.
	#[cfg(any(feature = "systemd_v245", feature = "dl_namespace"))]
	pub include_default_namespace: bool,
}

impl Default for JournalReaderConfig {
	fn default() -> JournalReaderConfig {
		JournalReaderConfig {
			files: JournalFiles::All,
			only_volatile: false,
			only_local: false,

			#[cfg(any(feature = "systemd_v245", feature = "dl_namespace"))]
			all_namespaces: false,

			#[cfg(any(feature = "systemd_v245", feature = "dl_namespace"))]
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

/// Wakeup event types
#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum WakeupType {
	NOP = 0,
	APPEND = 1,
	INVALIDATE = 2,
}

impl TryFrom<i32> for WakeupType {
	type Error = Error;

	fn try_from(value: i32) -> Result<Self> {
		Ok(match value {
			0 => WakeupType::NOP,
			1 => WakeupType::APPEND,
			2 => WakeupType::INVALIDATE,
			_ => return Err(Error::from_raw_os_error(libc::EINVAL)),
		})
	}
}

/// A reader for systemd journal.
///
/// Supports read, next, previous, and seek operations.
#[repr(transparent)]
pub struct JournalReader {
	#[cfg(feature = "libsystemd-sys")]
	j: *mut ffi::sd_journal,

	#[cfg(feature = "open")]
	j: *mut c_void,
}

impl JournalReader {
	/// Open the systemd journal for reading.
	pub fn open(config: &JournalReaderConfig) -> Result<JournalReader> {
		let mut flags: c_int = 0;

		if config.only_volatile {
			flags |= SD_JOURNAL_RUNTIME_ONLY;
		}

		if config.only_local {
			flags |= SD_JOURNAL_LOCAL_ONLY;
		}

		flags |= match config.files {
			JournalFiles::System => SD_JOURNAL_SYSTEM,
			JournalFiles::CurrentUser => SD_JOURNAL_CURRENT_USER,
			JournalFiles::All => 0,
		};

		let mut journal = JournalReader { j: ptr::null_mut() };

		#[cfg(feature = "libsystemd-sys")]
		unsafe {
			ffi_result(ffi::sd_journal_open(&mut journal.j, flags))?
		};

		#[cfg(feature = "open")]
		{
			let api = super::open_systemd()?;
			ffi_result(unsafe { api.sd_journal_open(&mut journal.j, flags) })?;
		}

		Ok(journal)
	}

	/// Open the systemd journal for reading from a specific namespace.
	#[cfg(any(feature = "systemd_v245", feature = "dl_namespace"))]
	pub fn open_namespace(config: &JournalReaderConfig, namespace: &str) -> Result<Self> {
		let mut flags: c_int = 0;

		if config.only_volatile {
			flags |= SD_JOURNAL_RUNTIME_ONLY;
		}

		if config.only_local {
			flags |= SD_JOURNAL_LOCAL_ONLY;
		}

		flags |= match config.files {
			JournalFiles::System => SD_JOURNAL_SYSTEM,
			JournalFiles::CurrentUser => SD_JOURNAL_CURRENT_USER,
			JournalFiles::All => 0,
		};

		if config.all_namespaces {
			flags |= SD_JOURNAL_ALL_NAMESPACES;
		}

		if config.include_default_namespace {
			flags |= SD_JOURNAL_INCLUDE_DEFAULT_NAMESPACE;
		}

		let mut journal = JournalReader { j: ptr::null_mut() };

		#[cfg(feature = "libsystemd-sys")]
		unsafe {
			ffi_result(ffi::sd_journal_open_namespace(
				&mut journal.j,
				::std::ffi::CString::new(namespace)?.as_ptr(),
				flags,
			))
		}?;

		#[cfg(feature = "open")]
		{
			let api = super::open_systemd()?;
			ffi_result(unsafe {
				api.sd_journal_open_namespace(
					&mut journal.j,
					std::ffi::CString::new(namespace)?.as_ptr(),
					flags,
				)
			})?;
		}

		Ok(journal)
	}
	/// Get and parse the currently journal entry from the journal
	/// It returns Result<Option<...>> out of convenience for calling
	/// functions. It always returns Ok(Some(...)) if successful.
	fn current_entry(&mut self) -> Result<Option<JournalEntry>> {
		#[cfg(feature = "libsystemd-sys")]
		unsafe {
			ffi::sd_journal_restart_data(self.j)
		}

		#[cfg(feature = "open")]
		let api = super::open_systemd()?;
		#[cfg(feature = "open")]
		unsafe {
			api.sd_journal_restart_data(self.j)
		};

		let mut fields = BTreeMap::new();
		let mut sz: size_t = 0;
		let mut data: *const u8 = ptr::null();

		while unsafe {
			#[cfg(feature = "libsystemd-sys")]
			{
				ffi::sd_journal_enumerate_data(self.j, &mut data as *mut *const u8, &mut sz)
			}

			#[cfg(feature = "open")]
			{
				api.sd_journal_enumerate_data(self.j, &mut data as *mut *const u8, &mut sz)
			}
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
		#[cfg(feature = "libsystemd-sys")]
		ffi_result(unsafe {
			ffi::sd_journal_get_realtime_usec(self.j, &mut timestamp_realtime_us)
		})?;

		#[cfg(feature = "open")]
		ffi_result(unsafe {
			api.sd_journal_get_realtime_usec(self.j, &mut timestamp_realtime_us)
		})?;

		fields.insert(
			"__REALTIME_TIMESTAMP".to_string(),
			timestamp_realtime_us.to_string(),
		);

		let mut timestamp_monotonic_us: u64 = 0;
		#[cfg(feature = "libsystemd-sys")]
		ffi_result(unsafe {
			ffi::sd_journal_get_monotonic_usec(self.j, &mut timestamp_monotonic_us, ptr::null_mut())
		})?;

		#[cfg(feature = "open")]
		ffi_result(unsafe {
			api.sd_journal_get_monotonic_usec(self.j, &mut timestamp_monotonic_us, ptr::null_mut())
		})?;

		fields.insert(
			"__MONOTONIC_TIMESTAMP".to_string(),
			timestamp_monotonic_us.to_string(),
		);

		let cursor;
		let mut b: *const c_char = ptr::null();
		#[cfg(feature = "libsystemd-sys")]
		ffi_result(unsafe { ffi::sd_journal_get_cursor(self.j, &mut b) })?;

		#[cfg(feature = "open")]
		ffi_result(unsafe { api.sd_journal_get_cursor(self.j, &mut b) })?;

		unsafe {
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
		#[cfg(feature = "libsystemd-sys")]
		if unsafe { ffi_result(ffi::sd_journal_next(self.j))? } == 0 {
			return Ok(None);
		}

		#[cfg(feature = "open")]
		if ffi_result(unsafe { super::open_systemd()?.sd_journal_next(self.j) })? == 0 {
			return Ok(None);
		}

		self.current_entry()
	}

	/// Read the previous entry from the journal. Returns `Ok(None)` if there
	/// are no more entries to read.
	pub fn previous_entry(&mut self) -> Result<Option<JournalEntry>> {
		#[cfg(feature = "libsystemd-sys")]
		if unsafe { ffi_result(ffi::sd_journal_previous(self.j))? } == 0 {
			return Ok(None);
		}

		#[cfg(feature = "open")]
		if ffi_result(unsafe { super::open_systemd()?.sd_journal_previous(self.j) })? == 0 {
			return Ok(None);
		}

		self.current_entry()
	}

	/// Seek to a specific position in journal. On success, returns a cursor
	/// to the current entry.
	pub fn seek(&mut self, seek: JournalSeek) -> Result<()> {
		match seek {
			JournalSeek::Head => unsafe {
				#[cfg(feature = "libsystemd-sys")]
				ffi_result(ffi::sd_journal_seek_head(self.j))?;

				#[cfg(feature = "open")]
				ffi_result(super::open_systemd()?.sd_journal_seek_head(self.j))?;
			},
			JournalSeek::Tail => unsafe {
				#[cfg(feature = "libsystemd-sys")]
				ffi_result(ffi::sd_journal_seek_tail(self.j))?;

				#[cfg(feature = "open")]
				ffi_result(super::open_systemd()?.sd_journal_seek_tail(self.j))?;
			},
			JournalSeek::Cursor(cur) => unsafe {
				let cur = ::std::ffi::CString::new(cur)?.as_ptr();

				#[cfg(feature = "libsystemd-sys")]
				ffi_result(ffi::sd_journal_seek_cursor(self.j, cur))?;
				#[cfg(feature = "open")]
				ffi_result(super::open_systemd()?.sd_journal_seek_cursor(self.j, cur))?;
			},
		};

		Ok(())
	}
	/// Sync wait until timeout for new journal messages
	pub fn wait_timeout(&mut self, timeout: Duration) -> Result<WakeupType> {
		self.wait_usec(duration_to_usec(timeout)?)
	}

	/// Sync wait forever for new journal messages
	pub fn wait(&mut self) -> Result<WakeupType> {
		self.wait_usec(u64::MAX)
	}

	/// Wait for an amount of usec
	pub(crate) fn wait_usec(&mut self, usec: u64) -> Result<WakeupType> {
		#[cfg(feature = "libsystemd-sys")]
		let ret = unsafe { ffi_result(ffi::sd_journal_wait(self.j, usec))? };

		#[cfg(feature = "open")]
		let ret = ffi_result(unsafe { super::open_systemd()?.sd_journal_wait(self.j, usec) })?;

		WakeupType::try_from(ret)
	}
	pub fn add_filter(&mut self, filter: &str) -> Result<()> {
		ffi_result(unsafe {
			#[cfg(feature = "libsystemd-sys")]
			{
				ffi::sd_journal_add_match(
					self.j,
					std::ffi::CString::new(filter)?.as_ptr() as *mut std::ffi::c_void,
					0,
				)
			}

			#[cfg(feature = "open")]
			{
				super::open_systemd()?.sd_journal_add_match(
					self.j,
					std::ffi::CString::new(filter)?.as_ptr() as *mut std::ffi::c_void,
					0,
				)
			}
		})?;

		Ok(())
	}

	/// Create a blocking Iterator from the reader.
	pub fn as_blocking_iter(&mut self) -> JournalBlockingIter {
		JournalBlockingIter {
			reader: self,
			timeout: u64::MAX,
		}
	}

	/// Create a blocking Iterator with a timeout of `timeout`.
	pub fn as_blocking_iter_timeout(&mut self, timeout: Duration) -> Result<JournalBlockingIter> {
		JournalBlockingIter::new(self, timeout)
	}

	/// Create a non blocking Iterator.
	pub fn as_iter(&mut self) -> JournalIter {
		JournalIter { reader: self }
	}
}

impl Drop for JournalReader {
	#[cfg(feature = "libsystemd-sys")]
	fn drop(&mut self) {
		if !self.j.is_null() {
			unsafe {
				ffi::sd_journal_close(self.j);
			}
		}
	}

	#[cfg(feature = "open")]
	fn drop(&mut self) {
		if !self.j.is_null() {
			let api = super::open_systemd().expect("Could not open api to remove systemd");
			unsafe { api.sd_journal_close(self.j) };
		}
	}
}

pub(crate) fn duration_to_usec(duration: Duration) -> Result<u64> {
	if duration.as_micros() > u64::MAX as u128 {
		return Err(Error::from_raw_os_error(libc::EOVERFLOW));
	}
	Ok(duration.as_micros() as u64)
}
