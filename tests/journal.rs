extern crate journald;
use std::time::{SystemTime, UNIX_EPOCH};

use journald::reader::*;
use journald::JournalEntry;

const TIME_EPSILON_SECONDS: i64 = 5;

const FILTER_FIELD: &str = "RUST_JOURNALD_TEST";

#[test]
fn test_reverse_walk() {
	let filter: String = format!("test_reverse_walk_{}", rand::random::<u64>());

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
		entry
			.fields
			.insert(FILTER_FIELD.to_string(), filter.to_string());
		journald::writer::submit(&entry).expect("journald write failed");
	}

	let mut journal =
		JournalReader::open(&JournalReaderConfig::default()).expect("journal open failed");

	journal
		.add_filter(&format!("{}={}", FILTER_FIELD, filter))
		.expect("Could not set journald filter");

	// give systemd internals some time
	std::thread::sleep(std::time::Duration::from_secs(1));

	journal
		.seek(JournalSeek::Tail)
		.expect("journal seek failed");

	for i in 1..(messages_expected.len() + 1) {
		let entry = journal
			.previous_entry()
			.expect("previous_entry failed")
			.expect("No entry found");

		let entry_message = entry.get_message().expect("No message").to_string();
		assert_eq!(
			entry_message,
			messages_expected[messages_expected.len() - i]
		);

		let entry_time = entry.get_wallclock_time().unwrap().timestamp_us / 1000000;

		assert!((entry_time - now_usec).abs() < TIME_EPSILON_SECONDS)
	}
}

#[test]
fn iter_blocking() {
	let now_usec: i64 = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap()
		.as_secs() as i64;

	let messages_expected = vec![
		"iter: rust-systemd test 1",
		"iter: rust-systemd test 2",
		"iter: rust-systemd test 3",
	];

	let mut journal =
		JournalReader::open(&JournalReaderConfig::default()).expect("journal open failed");

	std::thread::sleep(std::time::Duration::from_micros(1000));

	// we want a forward walk
	journal
		.seek(JournalSeek::Tail)
		.expect("journal seek failed");

	for message in &messages_expected {
		let mut entry = JournalEntry::new();
		entry.set_message(message);
		journald::writer::submit(&entry).expect("journald write failed");
	}

	// give systemd internals some time
	std::thread::sleep(std::time::Duration::from_micros(1000));

	let iter = journal.as_blocking_iter();

	let mut j = 0;
	let mut i = messages_expected.len();
	for entry in iter {
		let entry = entry.expect("failed to iterate");

		let entry_message = entry.get_message().unwrap().to_string();
		if !entry_message.starts_with("iter: ") {
			j += 1;
			// other logs should not create as many logs
			assert!(j < 20);
			continue;
		}

		let entry_time = entry.get_wallclock_time().unwrap().timestamp_us / 1000000;

		println!("{}: {}", entry_message, entry_time);

		assert_eq!(
			entry_message,
			messages_expected[messages_expected.len() - i]
		);

		let entry_time = entry.get_wallclock_time().unwrap().timestamp_us / 1000000;
		assert!((entry_time - now_usec).abs() < TIME_EPSILON_SECONDS);

		i -= 1;
		if i == 0 {
			return;
		}
	}
}
