use std::ptr::NonNull;

use crate::consts::DEFAULT_BUFFER_SIZE;
use crate::error::ResponseResult;
use crate::smart_object::SmartPtr;
use crate::FfmpegIOError;

const AVERROR_IO: i32 = ffmpeg_next::ffi::AVERROR(ffmpeg_next::ffi::EIO);

/// Safety: The function must be used with the same type as the one used to
/// generically create the function pointer
pub(crate) unsafe extern "C" fn read_packet<T: std::io::Read>(
	opaque: *mut libc::c_void,
	buf: *mut u8,
	buf_size: i32,
) -> i32 {
	let ret = (*(opaque as *mut T))
		.read(std::slice::from_raw_parts_mut(buf, buf_size as usize))
		.map(|n| n as i32)
		.unwrap_or(AVERROR_IO);

	if ret == 0 {
		return ffmpeg_next::ffi::AVERROR_EOF;
	}

	ret
}

/// Safety: The function must be used with the same type as the one used to
/// generically create the function pointer
pub(crate) unsafe extern "C" fn write_packet<T: std::io::Write>(
	opaque: *mut libc::c_void,
	buf: *mut u8,
	buf_size: i32,
) -> i32 {
	(*(opaque as *mut T))
		.write(std::slice::from_raw_parts(buf, buf_size as usize))
		.map(|n| n as i32)
		.unwrap_or(AVERROR_IO)
}

/// Safety: The function must be used with the same type as the one used to
/// generically create the function pointer
pub(crate) unsafe extern "C" fn seek<T: std::io::Seek>(opaque: *mut libc::c_void, offset: i64, mut whence: i32) -> i64 {
	let this = &mut *(opaque as *mut T);

	let seek_size = whence & ffmpeg_next::ffi::AVSEEK_SIZE != 0;
	if seek_size {
		whence &= !ffmpeg_next::ffi::AVSEEK_SIZE;
	}

	let seek_force = whence & ffmpeg_next::ffi::AVSEEK_FORCE != 0;
	if seek_force {
		whence &= !ffmpeg_next::ffi::AVSEEK_FORCE;
	}

	if seek_size {
		let Ok(pos) = this.stream_position() else {
			return AVERROR_IO as i64;
		};

		let Ok(end) = this.seek(std::io::SeekFrom::End(0)) else {
			return AVERROR_IO as i64;
		};

		if end != pos {
			let Ok(_) = this.seek(std::io::SeekFrom::Start(pos)) else {
				return AVERROR_IO as i64;
			};
		}

		return end as i64;
	}

	let whence = match whence {
		ffmpeg_next::ffi::SEEK_SET => std::io::SeekFrom::Start(offset as u64),
		ffmpeg_next::ffi::SEEK_CUR => std::io::SeekFrom::Current(offset),
		ffmpeg_next::ffi::SEEK_END => std::io::SeekFrom::End(offset),
		_ => return -1,
	};

	let ret = match this.seek(whence) {
		Ok(pos) => pos as i64,
		Err(_) => AVERROR_IO as i64,
	};

	ret
}

/// A helper struct to automatically deallocate all the resources allocated in C
pub(crate) struct InnerFormat<T> {
	pub(crate) context: SmartPtr<ffmpeg_next::ffi::AVFormatContext>,
	pub(crate) raw_input: Box<T>,
	pub(crate) _buffer: SmartPtr<libc::c_void>,
	pub(crate) _io_context: SmartPtr<ffmpeg_next::ffi::AVIOContext>,
}

pub(crate) struct InnerFormatOptions {
	pub(crate) buffer_size: usize,
	pub(crate) write_fn: Option<unsafe extern "C" fn(*mut libc::c_void, *mut u8, i32) -> i32>,
	pub(crate) read_fn: Option<unsafe extern "C" fn(*mut libc::c_void, *mut u8, i32) -> i32>,
	pub(crate) seek_fn: Option<unsafe extern "C" fn(*mut libc::c_void, i64, i32) -> i64>,
	pub(crate) output_format: *const ffmpeg_next::ffi::AVOutputFormat,
}

impl Default for InnerFormatOptions {
	fn default() -> Self {
		Self {
			buffer_size: DEFAULT_BUFFER_SIZE,
			write_fn: None,
			read_fn: None,
			seek_fn: None,
			output_format: std::ptr::null(),
		}
	}
}

impl<T> InnerFormat<T> {
	pub(crate) fn new(input: T, options: InnerFormatOptions) -> ResponseResult<Self, T> {
		let mut input = Box::new(input);

		// Safety: av_malloc is safe to call
		let Some(mut buffer) = NonNull::new(unsafe { ffmpeg_next::ffi::av_malloc(options.buffer_size) }).map(|ptr| {
			SmartPtr::new(ptr, |ptr| unsafe {
				// We own this resource so we need to free it
				ffmpeg_next::ffi::av_free((*ptr).as_ptr());
			})
		}) else {
			return Err((*input, FfmpegIOError::NullPointer("av_malloc")));
		};

		// Safety: avio_alloc_context is safe to call, and all the function pointers are
		// valid
		let Some(io) = NonNull::new(unsafe {
			ffmpeg_next::ffi::avio_alloc_context(
				buffer.as_ptr() as *mut u8,
				options.buffer_size as i32,
				if options.write_fn.is_some() { 1 } else { 0 },
				input.as_mut() as *mut _ as *mut libc::c_void,
				options.read_fn,
				options.write_fn,
				options.seek_fn,
			)
		})
		.map(|ptr| {
			SmartPtr::new(ptr, |ptr| unsafe {
				// We own these resources so we need to free them
				ffmpeg_next::ffi::av_free(ptr.as_mut().buffer as *mut libc::c_void);
				ffmpeg_next::ffi::avio_context_free(&mut ptr.as_ptr());
			})
		}) else {
			return Err((*input, FfmpegIOError::NullPointer("avio_alloc_context")));
		};

		// The buffer is now owned by the io context
		// The reason the buffer object no longer owns the buffer is because ffmpeg can
		// reallocate the buffer which would invalidate the pointer in the buffer
		// object.
		buffer.set_destructor(|_| {});

		let mut context = if options.write_fn.is_some() {
			let mut ptr = std::ptr::null_mut();

			// Safety: avformat_alloc_output_context2 is safe to call
			let ec = unsafe {
				ffmpeg_next::ffi::avformat_alloc_output_context2(
					&mut ptr,
					options.output_format,
					std::ptr::null(),
					std::ptr::null_mut(),
				)
			};
			if ec != 0 {
				return Err((*input, ffmpeg_next::Error::from(ec).into()));
			}

			// Safety: avformat_free_context is safe to call
			let Some(context) = NonNull::new(ptr).map(|ptr| {
				SmartPtr::new(ptr, |ptr| unsafe {
					// We own this resource so we need to free it
					ffmpeg_next::ffi::avformat_free_context(ptr.as_ptr());
				})
			}) else {
				return Err((*input, FfmpegIOError::NullPointer("avformat_alloc_output_context2")));
			};

			context
		} else {
			// Safety: avformat_alloc_context is safe to call
			let Some(context) = NonNull::new(unsafe { ffmpeg_next::ffi::avformat_alloc_context() }).map(|ptr| {
				SmartPtr::new(ptr, |ptr| unsafe {
					// We own this resource so we need to free it
					ffmpeg_next::ffi::avformat_free_context(ptr.as_ptr());
				})
			}) else {
				return Err((*input, FfmpegIOError::NullPointer("avformat_alloc_context")));
			};

			context
		};

		context.as_mut().pb = io.as_ptr();

		Ok(Self {
			context,
			raw_input: input,
			_buffer: buffer,
			_io_context: io,
		})
	}
}
