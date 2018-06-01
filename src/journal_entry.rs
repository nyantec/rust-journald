use std::collections::BTreeMap;

type JournalEntryFields = BTreeMap<String, String>;

pub struct JournalEntry {
	pub	fields: JournalEntryFields,
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

}
