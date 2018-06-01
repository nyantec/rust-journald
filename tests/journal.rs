extern crate systemd;
use systemd::journal;

#[test]
fn test_reverse_walk() {
	let messages_expected = vec![
		"rust-systemd test 1",
		"rust-systemd test 2",
		"rust-systemd test 3",
	];

	for message in &messages_expected {
		journal::send(&[&format!("MESSAGE={}", message)]);
	}

	let mut journal = journal::Journal
			::open(journal::JournalFiles::All, false, false)
			.expect("journal open failed");

	journal
			.seek(journal::JournalSeek::Tail)
			.expect("journal seek failed");

	let mut messages_actual = Vec::<String>::new();

	for _ in 0..3 {
		let entry = journal
				.previous_record()
				.expect("previous_record() failed");

		messages_actual.insert(0, entry.unwrap().get("MESSAGE").unwrap().to_string());
	}

	assert!(messages_expected == messages_actual);
}

