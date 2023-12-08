use imgref::{ImgRef, ImgVec};

pub type FrameRef<'a> = Frame<ImgRef<'a, rgb::RGBA8>>;
pub type FrameOwned = Frame<ImgVec<rgb::RGBA8>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<T> {
	pub image: T,
	pub duration_ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameCow<'a> {
	Ref(FrameRef<'a>),
	Owned(FrameOwned),
}

impl From<FrameOwned> for FrameCow<'_> {
	fn from(frame: FrameOwned) -> Self {
		Self::Owned(frame)
	}
}

impl<'a> From<FrameRef<'a>> for FrameCow<'a> {
	fn from(frame: FrameRef<'a>) -> Self {
		Self::Ref(frame)
	}
}

impl<'a> FrameCow<'a> {
	#[inline]
	pub fn as_ref(&self) -> FrameRef<'_> {
		match self {
			Self::Ref(frame) => *frame,
			Self::Owned(frame) => frame.as_ref(),
		}
	}

	#[inline]
	pub fn to_owned(self) -> FrameOwned {
		match self {
			Self::Ref(frame) => frame.to_owned(),
			Self::Owned(frame) => frame,
		}
	}
}

impl FrameOwned {
	#[inline]
	pub fn as_ref(&self) -> FrameRef<'_> {
		FrameRef {
			image: ImgRef::new(self.image.buf(), self.image.width(), self.image.height()),
			duration_ts: self.duration_ts,
		}
	}
}

impl FrameRef<'_> {
	#[inline]
	pub fn to_owned(self) -> FrameOwned {
		FrameOwned {
			image: ImgVec::new(self.image.buf().to_vec(), self.image.width(), self.image.height()),
			duration_ts: self.duration_ts,
		}
	}
}
