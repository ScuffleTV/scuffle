use byteorder::ReadBytesExt;
use rgb::ComponentBytes;
use sha2::Digest;

use super::frame::{FrameOwned, FrameRef};

pub struct FrameDeduplicator {
	previous_frame: Option<(u128, FrameOwned)>,
}

impl std::fmt::Debug for FrameDeduplicator {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("FrameDeduplicator").finish()
	}
}

impl FrameDeduplicator {
	pub const fn new() -> Self {
		Self { previous_frame: None }
	}

	pub fn deduplicate(&mut self, frame: FrameRef<'_>) -> Option<FrameOwned> {
		let hash = sha2::Sha256::digest(frame.image.buf().as_bytes());
		let hash = hash.as_slice().read_u128::<byteorder::BigEndian>().unwrap();

		if let Some((previous_hash, previous_frame)) = self.previous_frame.as_mut() {
			if *previous_hash == hash {
				previous_frame.duration_ts += frame.duration_ts;
			} else {
				*previous_hash = hash;
				return Some(std::mem::replace(previous_frame, frame.to_owned()));
			}
		} else {
			self.previous_frame = Some((hash, frame.to_owned()));
		}

		None
	}

	pub fn flush(&mut self) -> Option<FrameOwned> {
		self.previous_frame.take().map(|(_, frame)| frame)
	}
}
