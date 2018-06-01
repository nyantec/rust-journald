use std::collections::BTreeMap;

pub struct JournalEntry {
	pub	fields: BTreeMap<String, String>,
}

impl JournalEntry {

	pub fn new() -> JournalEntry {
		return JournalEntry {
			fields: BTreeMap::<String, String>::new(),
		};
	}

	pub fn from_fields(fields: &BTreeMap<String, String>) -> JournalEntry {
		return JournalEntry {
			fields: fields.clone(),
		};
	}

	pub fn get_message<'a>(&'a self) -> Option<&'a str> {
		return self
				.fields
				.get("MESSAGE")
				.map(|v| v.as_ref());
	}

}
