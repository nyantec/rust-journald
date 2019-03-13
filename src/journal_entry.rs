use std::collections::BTreeMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

type JournalEntryFields = BTreeMap<String, String>;

#[derive(Clone, Debug)]
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

	pub fn get_field(&self, field: &str) -> Option<&str> {
		self.fields.get(field).map(|v| v.as_str())
	}

	pub fn get_fields(&self) -> &JournalEntryFields {
		&self.fields
	}

	pub fn get_message(&self) -> Option<&str> {
		self.fields.get("MESSAGE").map(|v| v.as_ref())
	}

	pub fn set_message(&mut self, msg: &str) {
		self.fields.insert("MESSAGE".to_string(), msg.to_string());
	}

	pub fn get_wallclock_time(&self) -> Option<JournalEntryTimestamp> {
		let source_time = self.get_source_wallclock_time();
		let reception_time = self.get_reception_wallclock_time();

		source_time.or(reception_time)
	}

	pub fn get_source_wallclock_time(&self) -> Option<JournalEntryTimestamp> {
		self.fields
			.get("_SOURCE_REALTIME_TIMESTAMP")
			.and_then(|v| v.parse::<i64>().ok())
			.map(|v| JournalEntryTimestamp { timestamp_us: v })
	}

	pub fn get_reception_wallclock_time(&self) -> Option<JournalEntryTimestamp> {
		self.fields
			.get("__REALTIME_TIMESTAMP")
			.and_then(|v| v.parse::<i64>().ok())
			.map(|v| JournalEntryTimestamp { timestamp_us: v })
	}

	pub fn get_monotonic_time(&self) -> Option<JournalEntryTimestamp> {
		self.fields
			.get("__MONOTONIC_TIMESTAMP")
			.and_then(|v| v.parse::<i64>().ok())
			.map(|v| JournalEntryTimestamp { timestamp_us: v })
	}
}

impl Default for JournalEntry {
	fn default() -> Self {
		JournalEntry {
			fields: BTreeMap::<String, String>::new(),
		}
	}
}

impl From<&JournalEntryFields> for JournalEntry {
	fn from(fields: &JournalEntryFields) -> Self {
		JournalEntry {
			fields: fields.clone(),
		}
	}
}
