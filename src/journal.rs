use libc::{c_int, size_t};
use log::{self, Log, Record, Level, SetLoggerError};
use std::{ptr, result};
use std::collections::BTreeMap;
use ffi::array_to_iovecs;
use ffi::journal as ffi;
use super::Result;

/// Send preformatted fields to systemd.
///
/// This is a relatively low-level operation and probably not suitable unless
/// you need precise control over which fields are sent to systemd.
pub fn send(args: &[&str]) -> c_int {
	let iovecs = array_to_iovecs(args);
	unsafe { ffi::sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as c_int) }
}

/// Send a simple message to systemd-journald.
pub fn print(lvl: u32, s: &str) -> c_int {
	send(&[&format!("PRIORITY={}", lvl), &format!("MESSAGE={}", s)])
}

enum SyslogLevel {
	// Emerg = 0,
	// Alert = 1,
	// Crit = 2,
	Err = 3,
	Warning = 4,
	// Notice = 5,
	Info = 6,
	Debug = 7,
}


/// Send a `log::Record` to systemd-journald.
pub fn log_record(record: &Record) {
	let lvl = match record.level() {
		Level::Error => SyslogLevel::Err,
		Level::Warn => SyslogLevel::Warning,
		Level::Info => SyslogLevel::Info,
		Level::Debug |
		Level::Trace => SyslogLevel::Debug,
	} as usize;

	let mut keys = vec![
		format!("PRIORITY={}", lvl),
		format!("MESSAGE={}", record.args()),
		format!("TARGET={}", record.target()),
	];

	record.line().map(|line| keys.push(format!("CODE_LINE={}", line)));
	record.file().map(|file| keys.push(format!("CODE_FILE={}", file)));
	record.module_path().map(|module_path| keys.push(format!("CODE_FUNCTION={}", module_path)));

	let str_keys = keys.iter().map(AsRef::as_ref).collect::<Vec<_>>();
	send(&str_keys);
}



/// Logger implementation over systemd-journald.
pub struct JournalLog;
impl Log for JournalLog {
	fn enabled(&self, _metadata: &log::Metadata) -> bool {
		true
	}

	fn log(&self, record: &Record) {
		log_record(record);
	}

	fn flush(&self) {
		// There is no flushing required.
	}
}

static LOGGER: JournalLog = JournalLog;
impl JournalLog {
	pub fn init() -> result::Result<(), SetLoggerError> {
		log::set_logger(&LOGGER)
	}
}

// A single log entry from journal.
pub type JournalRecord = BTreeMap<String, String>;

/// A reader for systemd journal.
///
/// Supports read, next, previous, and seek operations.
pub struct Journal {
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

impl Journal {

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
	pub fn open(files: JournalFiles, runtime_only: bool, local_only: bool) -> Result<Journal> {
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

		let mut journal = Journal { j: ptr::null_mut() };
		sd_try!(ffi::sd_journal_open(&mut journal.j, flags));

		Ok(journal)
	}

	/// Get and parse the currently journal record from the journal
	/// It returns Result<Option<...>> out of convenience for calling
	/// functions. It always returns Ok(Some(...)) if successful.
	fn get_record(&mut self) -> Result<Option<JournalRecord>> {
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

	/// Read the next record from the journal. Returns `Ok(None)` if there
	/// are no more records to read.
	pub fn next_record(&mut self) -> Result<Option<JournalRecord>> {
		if sd_try!(ffi::sd_journal_next(self.j)) == 0 {
			return Ok(None);
		}

		return self.get_record();
	}

	/// Read the previous record from the journal. Returns `Ok(None)` if there
	/// are no more records to read.
	pub fn previous_record(&mut self) -> Result<Option<JournalRecord>> {
		if sd_try!(ffi::sd_journal_previous(self.j)) == 0 {
			return Ok(None);
		}
		
		return self.get_record();
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

impl Drop for Journal {

	fn drop(&mut self) {
		if !self.j.is_null() {
			unsafe {
				ffi::sd_journal_close(self.j);
			}
		}
	}

}
