use std::collections::BTreeMap;

type JournalEntryFields = BTreeMap<String, String>;

#[derive(Clone, Debug)]
pub struct JournalEntry {
	pub	fields: JournalEntryFields,
}

#[derive(Clone, Debug)]
pub struct JournalEntryTimestamp {
	pub timestamp_us: i64
}

impl JournalEntry {

	pub fn new() -> JournalEntry {
		return JournalEntry {
			fields: BTreeMap::<String, String>::new(),
		};
	}

	pub fn from_fields(fields: &JournalEntryFields) -> JournalEntry {
		return JournalEntry {
			fields: fields.clone(),
		};
	}

	pub fn get_field(&self, field: &str) -> Option<&str> {
		return self.fields.get(field).map(|v| v.as_str());
	}

	pub fn get_fields(&self) -> &JournalEntryFields {
		return &self.fields;
	}

	pub fn get_message<'a>(&'a self) -> Option<&'a str> {
		return self
				.fields
				.get("MESSAGE")
				.map(|v| v.as_ref());
	}

	pub fn set_message(&mut self, msg: &str) {
		self.fields.insert(
				"MESSAGE".to_string(),
				msg.to_string());
	}

	pub fn get_wallclock_time(&self) -> Option<JournalEntryTimestamp> {
		let source_time = self.get_source_wallclock_time();
		let reception_time = self.get_reception_wallclock_time();

		return source_time.or(reception_time);
	}

	pub fn get_source_wallclock_time(&self) -> Option<JournalEntryTimestamp> {
		return self
				.fields
				.get("_SOURCE_REALTIME_TIMESTAMP")
				.and_then(|v| v.parse::<i64>().ok())
				.map(|v| JournalEntryTimestamp { timestamp_us: v });
	}

	pub fn get_reception_wallclock_time(&self) -> Option<JournalEntryTimestamp> {
		return self
				.fields
				.get("__REALTIME_TIMESTAMP")
				.and_then(|v| v.parse::<i64>().ok())
				.map(|v| JournalEntryTimestamp { timestamp_us: v });
	}

	pub fn get_monotonic_time(&self) -> Option<JournalEntryTimestamp> {
		return self.fields
				.get("__MONOTONIC_TIMESTAMP")
				.and_then(|v| v.parse::<i64>().ok())
				.map(|v| JournalEntryTimestamp { timestamp_us: v });
	}

}
