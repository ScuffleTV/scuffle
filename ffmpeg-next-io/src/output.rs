use crate::consts::{DEFAULT_BUFFER_SIZE, MOVED_ERROR};
use crate::error::ResponseResult;
use crate::smart_object::SmartObject;
use crate::util::{seek, write_packet, InnerFormat, InnerFormatOptions};
use crate::{FfmpegIOError, Result};

#[derive(Debug, Clone)]
pub struct OutputOptions<'a> {
	pub buffer_size: usize,
	pub format_name: Option<&'a str>,
	pub format_mime_type: Option<&'a str>,
	pub format_ffi: *const ffmpeg_next::ffi::AVOutputFormat,
}

impl OutputOptions<'_> {
	fn format_ffi(&self) -> Result<*const ffmpeg_next::ffi::AVOutputFormat> {
		if !self.format_ffi.is_null() {
			return Ok(self.format_ffi);
		}

		if self.format_name.is_none() && self.format_mime_type.is_none() {
			return Err(FfmpegIOError::Arguments(
				"format_ffi, format_name and format_mime_type cannot all be unset",
			));
		}

		let c_format_name = self.format_name.map(|s| std::ffi::CString::new(s).unwrap());
		let c_format_mime_type = self.format_mime_type.map(|s| std::ffi::CString::new(s).unwrap());
		let c_format_name_ptr = c_format_name.as_ref().map(|s| s.as_ptr()).unwrap_or(std::ptr::null());
		let c_format_mime_type_ptr = c_format_mime_type.as_ref().map(|s| s.as_ptr()).unwrap_or(std::ptr::null());

		let output_format =
			unsafe { ffmpeg_next::ffi::av_guess_format(c_format_name_ptr, std::ptr::null(), c_format_mime_type_ptr) };

		if output_format.is_null() {
			return Err(FfmpegIOError::Arguments("could not determine output format"));
		}

		Ok(output_format)
	}
}

impl Default for OutputOptions<'_> {
	fn default() -> Self {
		Self {
			buffer_size: DEFAULT_BUFFER_SIZE,
			format_name: None,
			format_mime_type: None,
			format_ffi: std::ptr::null(),
		}
	}
}

pub struct Output<T: Send + Sync> {
	output: SmartObject<ffmpeg_next::format::context::Output>,
	_inner: InnerFormat<T>,
}

impl<T: Send + Sync> Output<T> {
	pub fn into_inner(self) -> T {
		*self._inner.raw_input
	}
}

impl<T: std::io::Write + Send + Sync> Output<T> {
	pub fn new(input: T, options: OutputOptions) -> ResponseResult<Self, T> {
		let output_format = match options.format_ffi() {
			Ok(format) => format,
			Err(e) => return Err((input, e)),
		};

		Self::create_output(InnerFormat::new(
			input,
			InnerFormatOptions {
				buffer_size: options.buffer_size,
				write_fn: Some(write_packet::<T>),
				output_format,
				..Default::default()
			},
		)?)
	}

	pub fn seekable(input: T, options: OutputOptions) -> ResponseResult<Self, T>
	where
		T: std::io::Seek,
	{
		let output_format = match options.format_ffi() {
			Ok(format) => format,
			Err(e) => return Err((input, e)),
		};

		Self::create_output(InnerFormat::new(
			input,
			InnerFormatOptions {
				buffer_size: options.buffer_size,
				write_fn: Some(write_packet::<T>),
				seek_fn: Some(seek::<T>),
				output_format,
				..Default::default()
			},
		)?)
	}

	fn create_output(mut inner: InnerFormat<T>) -> ResponseResult<Self, T> {
		// Safety: Input is now the owner of the context, and it frees it when it is
		// dropped
		let output = unsafe { ffmpeg_next::format::context::Output::wrap(inner.context.as_ptr()) };

		// Output now owns the context, safety is guaranteed by the above comment
		inner.context.set_destructor(|_| {});

		Ok(Self {
			output: SmartObject::new(output, |ptr| unsafe {
				// Before we drop the output we need to unset the pb pointer because it is now
				// owned by this object
				ptr.as_mut_ptr().as_mut().unwrap().pb = std::ptr::null_mut();
			}),
			_inner: inner,
		})
	}
}

impl<T: Send + Sync> std::ops::Deref for Output<T> {
	type Target = ffmpeg_next::format::context::Output;

	fn deref(&self) -> &Self::Target {
		debug_assert_eq!(
			unsafe { self.output.as_ptr() } as usize,
			self._inner.context.as_ptr() as usize,
			"{MOVED_ERROR}"
		);
		&self.output
	}
}

impl<T: Send + Sync> std::ops::DerefMut for Output<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		debug_assert_eq!(
			unsafe { self.output.as_ptr() } as usize,
			self._inner.context.as_ptr() as usize,
			"{MOVED_ERROR}"
		);
		&mut self.output
	}
}
