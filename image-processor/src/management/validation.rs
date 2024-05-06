use std::sync::Arc;

use scuffle_image_processor_proto::{
	input, DrivePath, Error, ErrorCode, EventQueue, Events, Input, InputMetadata, Limits, Output, OutputVariants, Task,
};
use url::Url;

use crate::global::Global;

#[derive(Debug, Clone, Copy)]
pub enum FragmentItem {
	Map(&'static str),
	Index(usize),
}

#[derive(Debug)]
pub struct FragmentBuf {
	path: Vec<FragmentItem>,
}

impl FragmentBuf {
	pub fn new() -> Self {
		Self { path: Vec::new() }
	}

	pub fn push<'a>(&'a mut self, path: impl Into<FragmentItem>) -> Fragment<'a> {
		self.path.push(path.into());
		Fragment::new(&mut self.path)
	}

	pub fn as_fagment(&mut self) -> Fragment {
		Fragment::new(&mut self.path)
	}
}

#[derive(Debug)]
pub struct Fragment<'a> {
	path: &'a mut Vec<FragmentItem>,
}

impl<'a> Fragment<'a> {
	pub fn new(path: &'a mut Vec<FragmentItem>) -> Self {
		Self { path }
	}
}

impl From<&'static str> for FragmentItem {
	fn from(value: &'static str) -> Self {
		Self::Map(value)
	}
}

impl From<usize> for FragmentItem {
	fn from(value: usize) -> Self {
		Self::Index(value)
	}
}

// This is a bit of a hack to allow us to convert from a reference to a copy.
// &&'static str -> &'static str -> FragmentItem
// &usize -> usize -> FragmentItem
impl<T> From<&T> for FragmentItem
	where
		T: Copy,
		FragmentItem: From<T>,
{
	fn from(value: &T) -> Self {
		Self::from(*value)
	}
}

impl std::fmt::Display for Fragment<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut first = true;
		for item in self.path.iter() {
			match item {
				FragmentItem::Map(value) => {
					if first {
						write!(f, ".")?;
					}
					write!(f, "{value}")?;
				}
				FragmentItem::Index(value) => {
					write!(f, "[{value}]")?;
				}
			}

			first = false;
		}

		Ok(())
	}
}

impl Fragment<'_> {
	pub fn push<'a>(&'a mut self, path: impl Into<FragmentItem>) -> Fragment<'a> {
		self.path.push(path.into());
		Fragment::new(self.path)
	}
}

impl Drop for Fragment<'_> {
	fn drop(&mut self) {
		self.path.pop();
	}
}

pub fn validate_task(global: &Arc<Global>, mut fragment: Fragment, task: Option<&Task>) -> Result<(), Error> {
	let task = task.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	validate_input(global, fragment.push("input"), task.input.as_ref())?;

	validate_output(global, fragment.push("output"), task.output.as_ref())?;

	validate_events(global, fragment.push("events"), task.events.as_ref())?;

	if let Some(limits) = &task.limits {
		validate_limits(fragment.push("limits"), Some(limits))?;
	}

	Ok(())
}

fn validate_limits(mut fragment: Fragment, limits: Option<&Limits>) -> Result<(), Error> {
	let limits = limits.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	let fields = [
		(limits.max_processing_time_ms, "max_processing_time_ms"),
		(limits.max_input_frame_count, "max_input_frame_count"),
		(limits.max_input_width, "max_input_width"),
		(limits.max_input_height, "max_input_height"),
		(limits.max_input_duration_ms, "max_input_duration_ms"),
	];

	for (value, name) in &fields {
		if let Some(0) = value {
			return Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!("{}: must be non 0", fragment.push(name)),
			});
		}
	}

	Ok(())
}

fn validate_events(global: &Arc<Global>, mut fragment: Fragment, events: Option<&Events>) -> Result<(), Error> {
	let events = events.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	let events = [
		(events.on_success.as_ref(), "on_success"),
		(events.on_failure.as_ref(), "on_failure"),
		(events.on_cancel.as_ref(), "on_cancel"),
		(events.on_start.as_ref(), "on_start"),
	];

	for (event, name) in &events {
		if let Some(event) = event {
			validate_event_queue(global, fragment.push(name), Some(event))?;
		}
	}

	Ok(())
}

fn validate_event_queue(global: &Arc<Global>, mut fragment: Fragment, event: Option<&EventQueue>) -> Result<(), Error> {
	let event_queue = event.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	if event_queue.name.is_empty() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: is required", fragment.push("name")),
		});
	}

	if global.event_queue(&event_queue.name).is_none() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{fragment}: event queue not found"),
		});
	}

	// Validate the topic template string
	validate_template_string(fragment.push("topic"), &["id"], &event_queue.topic)?;

	Ok(())
}

pub fn validate_output(global: &Arc<Global>, mut fragment: Fragment, output: Option<&Output>) -> Result<(), Error> {
	let output = output.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	validate_drive_path(global, fragment.push("path"), output.drive_path.as_ref())?;

	validate_output_variants(fragment.push("variants"), output.variants.as_ref())?;

	Ok(())
}

pub fn validate_output_variants(mut fragment: Fragment, variants: Option<&OutputVariants>) -> Result<(), Error> {
	let variants = variants.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	validate_template_string(
		fragment.push("suffix"),
		&[
			"id",
			"format",
			"scale",
			"width",
			"height",
			"format_idx",
			"resize_idx",
			"static",
			"ext",
		],
		&variants.suffix,
	)?;

	if variants.formats.is_empty() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: is required", fragment.push("formats")),
		});
	}

	for (idx, format) in variants.formats.iter().enumerate() {}

	Ok(())
}

pub fn validate_input(global: &Arc<Global>, mut fragment: Fragment, input: Option<&Input>) -> Result<(), Error> {
	let input = input.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	validate_input_path(global, fragment.push("path"), input.path.as_ref())?;

	// Metadata is optional
	if let Some(metadata) = &input.metadata {
		validate_input_metadata(fragment.push("metadata"), Some(metadata))?;
	}

	Ok(())
}

pub fn validate_input_metadata(mut fragment: Fragment, metadata: Option<&InputMetadata>) -> Result<(), Error> {
	let metadata = metadata.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{} is required", fragment),
	})?;

	match (metadata.static_frame_index, metadata.frame_count) {
		(None, Some(frame_count)) if frame_count == 0 => {
			return Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!("{}: frame_count must be non 0", fragment),
			});
		}
		(Some(static_frame_index), Some(frame_count)) if static_frame_index >= frame_count => {
			return Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!(
					"{}: static_frame_index must be less than frame_count, {static_frame_index} >= {frame_count}",
					fragment
				),
			});
		}
		(Some(_), None) => {
			return Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!(
					"{}: is required when static_frame_index is provided",
					fragment.push("frame_count")
				),
			});
		}
		_ => {}
	}

	if metadata.width == 0 {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: width must be non 0", fragment.push("width")),
		});
	}

	if metadata.height == 0 {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: height must be non 0", fragment.push("height")),
		});
	}

	Ok(())
}

pub fn validate_input_path(
	global: &Arc<Global>,
	mut fragment: Fragment,
	input_path: Option<&input::Path>,
) -> Result<(), Error> {
	let input_path = input_path.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{} is required", fragment),
	})?;

	match input_path {
		input::Path::DrivePath(drive_path) => {
			validate_drive_path(global, fragment.push("drive_path"), Some(drive_path))?;
		}
		input::Path::PublicUrl(url) => {
			validate_public_url(global, fragment.push("public_url"), url)?;
		}
	}

	Ok(())
}

pub fn validate_drive_path(
	global: &Arc<Global>,
	mut fragment: Fragment,
	drive_path: Option<&DrivePath>,
) -> Result<(), Error> {
	let drive_path = drive_path.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{} is required", fragment),
	})?;

	if global.drive(&drive_path.drive).is_none() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: drive not found", fragment.push("drive")),
		});
	}

	validate_template_string(fragment.push("path"), &["id"], &drive_path.path)?;

	Ok(())
}

pub fn validate_public_url(global: &Arc<Global>, fragment: Fragment, url: &str) -> Result<(), Error> {
	if url.is_empty() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{fragment}: is required"),
		});
	} else if global.public_http_drive().is_none() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{fragment}: public http drive not found"),
		});
	}

	let url = Url::parse(url).map_err(|e| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: {e}"),
	})?;

	if url.scheme() != "http" && url.scheme() != "https" {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{fragment}: scheme must be http or https"),
		});
	}

	if url.host().is_none() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{fragment}: url host is required"),
		});
	}

	Ok(())
}

fn validate_template_string(fragment: Fragment, allowed_vars: &[&str], template: &str) -> Result<String, Error> {
	if template.is_empty() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{fragment}: is required"),
		});
	}

	let formatter = |fmt: strfmt::Formatter| {
		let k: &str = fmt.key;
		if !allowed_vars.contains(&k) {
			return Err(strfmt::FmtError::KeyError(k.to_owned()));
		}
		Ok(())
	};

	strfmt::strfmt_map(template, formatter).map_err(|err| match err {
		strfmt::FmtError::KeyError(key) => Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!(
				"{fragment}: invalid variable '{key}', the allowed variables are {:?}",
				allowed_vars
			),
		},
		strfmt::FmtError::TypeError(_) | strfmt::FmtError::Invalid(_) => Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{fragment}: invalid template syntax"),
		},
	})
}
