use std::io::Result;
use std::time::Duration;

use crate::reader::{JournalReader, WakeupType};
use crate::JournalEntry;

/// A blocking iter over the journald entries.
/// This blocks the thread if there are currently no new
/// entries from journald
pub struct JournalBlockingIter<'a> {
	pub(crate) reader: &'a mut JournalReader,
	pub(crate) timeout: u64,
}

impl<'a> JournalBlockingIter<'a> {
	/// Set a duration as timeout for the Iterator.
	///
	/// The timeout is internally stored as u64 in micros. There as the [`timeout.as_mircos()`]
	/// cannot be larger than [`u64::MAX`]. Returns [`libc::EOVERFLOW`] in that case.
	pub fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
		self.timeout = super::duration_to_usec(timeout)?;
		Ok(())
	}

	/// Get the current set timeout of the iterator.
	pub fn get_timeout(&self) -> Duration {
		Duration::from_micros(self.timeout)
	}

	pub(crate) fn new(reader: &'a mut JournalReader, timeout: Duration) -> Result<Self> {
		Ok(Self {
			reader,
			timeout: super::duration_to_usec(timeout)?,
		})
	}

	fn next_wait(&mut self) -> Result<Option<JournalEntry>> {
		let ret = self.reader.next_entry();

		if ret.is_ok() && ret.as_ref().unwrap().is_none() {
			let wakeup = self.reader.wait_usec(self.timeout)?;
			if wakeup != WakeupType::NOP {
				return self.next_wait();
			}
			log::trace!("got WakeupType '{:?}' from systemd in BlockingIter", wakeup);
			return self.reader.next_entry();
		}

		ret
	}
}

impl<'a> Iterator for JournalBlockingIter<'a> {
	type Item = Result<JournalEntry>;

	fn next(&mut self) -> Option<Self::Item> {
		self.next_wait().transpose()
	}
}

		let ret = if ret.is_ok() && ret.as_ref().unwrap().is_none() {
			if let Err(e) = self.reader.wait() {
				return Some(Err(e));
			}
			self.reader.next_entry()
		} else {
			ret
		};

		ret.transpose()
	}
}
