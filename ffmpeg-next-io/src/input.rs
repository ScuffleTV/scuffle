use crate::consts::{DEFAULT_BUFFER_SIZE, MOVED_ERROR};
use crate::error::ResponseResult;
use crate::util::{read_packet, seek, InnerFormat, InnerFormatOptions};

pub struct Input<T: Send + Sync> {
	input: ffmpeg_next::format::context::Input,
	_inner: InnerFormat<T>,
}

impl<T: Send + Sync> Input<T> {
	pub fn into_inner(self) -> T {
		*self._inner.raw_input
	}
}

#[derive(Debug, Clone)]
pub struct InputOptions<'a, I: FnMut() -> bool> {
	pub buffer_size: usize,
	pub dictionary: ffmpeg_next::Dictionary<'a>,
	pub interrupt_callback: Option<I>,
}

impl Default for InputOptions<'_, fn() -> bool> {
	fn default() -> Self {
		Self {
			buffer_size: DEFAULT_BUFFER_SIZE,
			dictionary: ffmpeg_next::Dictionary::new(),
			interrupt_callback: None,
		}
	}
}

impl<T: std::io::Read + Send + Sync> Input<T> {
	pub fn new(input: T) -> ResponseResult<Self, T> {
		Self::with_options(input, InputOptions::default())
	}

	pub fn with_options(input: T, options: InputOptions<'_, impl FnMut() -> bool>) -> ResponseResult<Self, T> {
		Self::create_input(
			InnerFormat::new(
				input,
				InnerFormatOptions {
					buffer_size: options.buffer_size,
					read_fn: Some(read_packet::<T>),
					..Default::default()
				},
			)?,
			options.dictionary,
		)
	}

	pub fn seekable(input: T) -> ResponseResult<Self, T>
	where
		T: std::io::Seek,
	{
		Self::seekable_with_options(input, InputOptions::default())
	}

	pub fn seekable_with_options(input: T, options: InputOptions<'_, impl FnMut() -> bool>) -> ResponseResult<Self, T>
	where
		T: std::io::Seek,
	{
		Self::create_input(
			InnerFormat::new(
				input,
				InnerFormatOptions {
					buffer_size: options.buffer_size,
					read_fn: Some(read_packet::<T>),
					seek_fn: Some(seek::<T>),
					..Default::default()
				},
			)?,
			options.dictionary,
		)
	}

	fn create_input(mut inner: InnerFormat<T>, dictionary: ffmpeg_next::Dictionary<'_>) -> ResponseResult<Self, T> {
		// Safety we don't return from this function without taking back the ownership
		// of the dictionary
		let mut dict_ptr = unsafe { dictionary.disown() };

		// Safety: avformat_open_input is safe to call
		let ec = unsafe {
			ffmpeg_next::ffi::avformat_open_input(
				&mut inner.context.as_ptr(),
				std::ptr::null(),
				std::ptr::null(),
				&mut dict_ptr,
			)
		};

		// Safety: We own the dictionary, and we are responsible for freeing it
		unsafe { ffmpeg_next::Dictionary::own(dict_ptr) };

		if ec != 0 {
			return Err((*inner.raw_input, ffmpeg_next::Error::from(ec).into()));
		}

		// Safety: avformat_find_stream_info is safe to call
		let ec = unsafe { ffmpeg_next::ffi::avformat_find_stream_info(inner.context.as_ptr(), std::ptr::null_mut()) };
		if ec < 0 {
			// We need to close the input here if we fail above.
			// Since the input wrap below has not taken ownership of the context yet, we
			// need to close it manually.
			unsafe { ffmpeg_next::ffi::avformat_close_input(&mut inner.context.as_ptr()) };
			return Err((*inner.raw_input, ffmpeg_next::Error::from(ec).into()));
		}

		// Safety: Input is now the owner of the context, and it frees it when it is
		// dropped
		let input = unsafe { ffmpeg_next::format::context::Input::wrap(inner.context.as_ptr()) };

		// The context is now owned by the input, safety is guaranteed by the above
		// comment
		inner.context.set_destructor(|_| {});

		Ok(Self { input, _inner: inner })
	}
}

impl<T: Send + Sync> std::ops::Deref for Input<T> {
	type Target = ffmpeg_next::format::context::Input;

	fn deref(&self) -> &Self::Target {
		debug_assert_eq!(
			unsafe { self.input.as_ptr() } as usize,
			self._inner.context.as_ptr() as usize,
			"{MOVED_ERROR}"
		);
		&self.input
	}
}

impl<T: Send + Sync> std::ops::DerefMut for Input<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		debug_assert_eq!(
			unsafe { self.input.as_ptr() } as usize,
			self._inner.context.as_ptr() as usize,
			"{MOVED_ERROR}"
		);
		&mut self.input
	}
}
