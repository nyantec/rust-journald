extern crate journald;
use std::time::{SystemTime, UNIX_EPOCH};

use journald::reader::*;
use journald::JournalEntry;

const TIME_EPSILON_SECONDS: i64 = 5;

#[test]
fn test_reverse_walk() {
	let now_usec: i64 = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap()
		.as_secs() as i64;

	let messages_expected = vec![
		"rust-systemd test 1",
		"rust-systemd test 2",
		"rust-systemd test 3",
	];

	for message in &messages_expected {
		let mut entry = JournalEntry::new();
		entry.set_message(message);
		journald::writer::submit(&entry).expect("journald write failed");
	}

	let mut journal =
		JournalReader::open(&JournalReaderConfig::default()).expect("journal open failed");

	journal
		.seek(JournalSeek::Tail)
		.expect("journal seek failed");

	for i in 1..(messages_expected.len() + 1) {
		let entry = journal
			.previous_entry()
			.expect("previous_record() failed")
			.unwrap();

		let entry_message = entry.get_message().unwrap().to_string();
		assert!(entry_message == messages_expected[messages_expected.len() - i]);

		let entry_time = entry.get_wallclock_time().unwrap().timestamp_us / 1000000;

		assert!((entry_time - now_usec).abs() < TIME_EPSILON_SECONDS)
	}
}
