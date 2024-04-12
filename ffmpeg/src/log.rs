use std::ffi::CStr;
use std::sync::RwLock;

use ffmpeg_sys_next::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum LogLevel {
	Quiet = AV_LOG_QUIET,
	Panic = AV_LOG_PANIC,
	Fatal = AV_LOG_FATAL,
	Error = AV_LOG_ERROR,
	Warning = AV_LOG_WARNING,
	Info = AV_LOG_INFO,
	Verbose = AV_LOG_VERBOSE,
	Debug = AV_LOG_DEBUG,
	Trace = AV_LOG_TRACE,
}

impl LogLevel {
	pub const fn from_i32(value: i32) -> Self {
		match value {
			AV_LOG_QUIET => Self::Quiet,
			AV_LOG_PANIC => Self::Panic,
			AV_LOG_FATAL => Self::Fatal,
			AV_LOG_ERROR => Self::Error,
			AV_LOG_WARNING => Self::Warning,
			AV_LOG_INFO => Self::Info,
			AV_LOG_VERBOSE => Self::Verbose,
			AV_LOG_DEBUG => Self::Debug,
			AV_LOG_TRACE => Self::Trace,
			_ => Self::Info,
		}
	}

	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Quiet => "quiet",
			Self::Panic => "panic",
			Self::Fatal => "fatal",
			Self::Error => "error",
			Self::Warning => "warning",
			Self::Info => "info",
			Self::Verbose => "verbose",
			Self::Debug => "debug",
			Self::Trace => "trace",
		}
	}
}

pub fn set_log_level(level: LogLevel) {
	unsafe {
		av_log_set_level(level as i32);
	}
}

pub fn log_callback_set<F: Fn(LogLevel, Option<String>, String) + 'static>(callback: F) {
	type Function = Box<dyn Fn(LogLevel, Option<String>, String)>;
	static mut LOG_CALLBACK: RwLock<Option<Function>> = RwLock::new(None);

	unsafe extern "C" fn log_cb(
		ptr: *mut libc::c_void,
		level: libc::c_int,
		fmt: *const libc::c_char,
		va: *mut __va_list_tag,
	) {
		let level = LogLevel::from_i32(level);
		let class = if ptr.is_null() {
			None
		} else {
			let class = &mut **(ptr as *mut *mut AVClass);
			class
				.item_name
				.map(|im| CStr::from_ptr(im(ptr)).to_string_lossy().trim().to_owned())
		};

		let mut buf = [0u8; 1024];

		vsnprintf(buf.as_mut_ptr() as *mut i8, buf.len() as _, fmt, va);

		let msg = CStr::from_ptr(buf.as_ptr() as *const i8).to_string_lossy().trim().to_owned();

		(LOG_CALLBACK.read().unwrap().as_deref().unwrap())(level, class, msg)
	}

	unsafe {
		*LOG_CALLBACK.write().unwrap() = Some(Box::new(callback));
		av_log_set_callback(Some(log_cb));
	}
}

pub fn log_callback_unset() {
	unsafe {
		av_log_set_callback(None);
	}
}

#[cfg(feature = "tracing")]
pub fn log_callback_tracing() {
	log_callback_set(|mut level, class, msg| {
		let class = class.unwrap_or_else(|| "ffmpeg".to_owned());

		if msg == "deprecated pixel format used, make sure you did set range correctly" {
			level = LogLevel::Debug;
		}

		match level {
			LogLevel::Trace => tracing::trace!("{}: {class} @ {msg}", level.as_str()),
			LogLevel::Verbose => tracing::trace!("{}: [{class} @ {msg}", level.as_str()),
			LogLevel::Debug => tracing::trace!("{}: {class} @ {msg}", level.as_str()),
			LogLevel::Info => tracing::debug!("{}: {class} @ {msg}", level.as_str()),
			LogLevel::Warning => tracing::info!("{}: {class} @ {msg}", level.as_str()),
			LogLevel::Quiet => tracing::error!("{}: {class} @ {msg}", level.as_str()),
			LogLevel::Error => tracing::error!("{}: {class} @ {msg}", level.as_str()),
			LogLevel::Panic => tracing::error!("{}: {class} @ {msg}", level.as_str()),
			LogLevel::Fatal => tracing::error!("{}: {class} @ {msg}", level.as_str()),
		}
	});
}
