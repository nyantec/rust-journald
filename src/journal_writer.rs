use libc::c_int;
use log::{self, Log, Record, Level, SetLoggerError};
use std::result;
use ffi::array_to_iovecs;
use ffi::journal as ffi;

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

