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

	// give systemd internals some time
	std::thread::sleep(std::time::Duration::from_micros(1000));

	journal
		.seek(JournalSeek::Tail)
		.expect("journal seek failed");

	println!("time: {}", now_usec);

	let mut i = 1;
	let mut j = 1;
	loop {
		j += 1;
		let entry = journal
			.previous_entry()
			.expect("previous_record() failed")
			.unwrap();

		let entry_message = entry.get_message().unwrap().to_string();
		if !entry_message.starts_with("rust-systemd") {
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

		i += 1;
		if i == messages_expected.len() + 1 {
			return;
		}
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
