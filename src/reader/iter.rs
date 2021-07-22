use std::io::Result;

use crate::reader::JournalReader;
use crate::JournalEntry;

/// A blocking iter over the journald entries.
/// This blocks the thread if there are currently no new
/// entries from journald
pub struct JournalBlockingIter<'a> {
	pub(crate) reader: &'a mut JournalReader,
}

impl<'a> Iterator for JournalBlockingIter<'a> {
	type Item = Result<JournalEntry>;

	fn next(&mut self) -> Option<Self::Item> {
		let ret = self.reader.next_entry();

		let ret = if ret.is_ok() && ret.as_ref().unwrap().is_none() {
			self.reader.wait();
			self.reader.next_entry()
		} else {
			ret
		};

		ret.transpose()
	}
}
