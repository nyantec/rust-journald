extern crate journald;
use journald::reader::*;
use journald::JournalEntry;

#[test]
fn test_reverse_walk() {
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

	let mut journal = JournalReader::open(&JournalReaderConfig::default())
			.expect("journal open failed");

	journal
			.seek(JournalSeek::Tail)
			.expect("journal seek failed");

	let mut messages_actual = Vec::<String>::new();

	for _ in 0..3 {
		let entry = journal
				.previous_entry()
				.expect("previous_record() failed")
				.unwrap();

		messages_actual.insert(0, entry.get_message().unwrap().to_string());
	}

	assert!(messages_expected == messages_actual);
}

