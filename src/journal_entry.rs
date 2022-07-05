use std::borrow::Cow;
use std::collections::BTreeMap;
use std::str;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

type JournalEntryFields = BTreeMap<String, Vec<u8>>;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct JournalEntry {
	pub fields: JournalEntryFields,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct JournalEntryTimestamp {
	pub timestamp_us: i64,
}

impl JournalEntry {
	pub fn new() -> Self {
		Self::default()
	}

	#[deprecated(since = "0.7.0", note = "use `JournalEntry::from`")]
	pub fn from_fields(fields: &JournalEntryFields) -> JournalEntry {
		Self::from(fields)
	}

	pub fn get_field_binary(&self, field: &str) -> Option<&[u8]> {
		self.fields.get(field).map(|v| v.as_slice())
	}

	pub fn get_field_string(&self, field: &str) -> Result<Option<&str>, str::Utf8Error> {
		self.fields
			.get(field)
			.map(|v| str::from_utf8(v))
			.transpose()
	}

	pub fn get_field_string_lossy(&self, field: &str) -> Option<Cow<'_, str>> {
		self.fields.get(field).map(|v| String::from_utf8_lossy(v))
	}

	pub fn get_fields(&self) -> &JournalEntryFields {
		&self.fields
	}

	pub fn get_message(&self) -> Option<Cow<'_, str>> {
		self.get_field_string_lossy("MESSAGE")
	}

	pub fn set_message(&mut self, msg: &str) {
		self.fields
			.insert("MESSAGE".to_string(), msg.as_bytes().to_vec());
	}

	pub fn get_wallclock_time(&self) -> Option<JournalEntryTimestamp> {
		let source_time = self.get_source_wallclock_time();
		let reception_time = self.get_reception_wallclock_time();

		source_time.or(reception_time)
	}

	pub fn get_source_wallclock_time(&self) -> Option<JournalEntryTimestamp> {
		self.get_field_string_lossy("_SOURCE_REALTIME_TIMESTAMP")
			.and_then(|v| v.parse::<i64>().ok())
			.map(|v| JournalEntryTimestamp { timestamp_us: v })
	}

	pub fn get_reception_wallclock_time(&self) -> Option<JournalEntryTimestamp> {
		self.get_field_string_lossy("__REALTIME_TIMESTAMP")
			.and_then(|v| v.parse::<i64>().ok())
			.map(|v| JournalEntryTimestamp { timestamp_us: v })
	}

	pub fn get_monotonic_time(&self) -> Option<JournalEntryTimestamp> {
		self.get_field_string_lossy("__MONOTONIC_TIMESTAMP")
			.and_then(|v| v.parse::<i64>().ok())
			.map(|v| JournalEntryTimestamp { timestamp_us: v })
	}
}

impl From<&JournalEntryFields> for JournalEntry {
	fn from(fields: &JournalEntryFields) -> Self {
		JournalEntry {
			fields: fields.clone(),
		}
	}
}
