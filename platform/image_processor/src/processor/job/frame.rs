use imgref::ImgVec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
	pub image: ImgVec<rgb::RGBA8>,
	pub duration_ts: u64,
}
