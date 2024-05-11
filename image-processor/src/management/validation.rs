use std::collections::HashSet;
use std::sync::Arc;

use scuffle_image_processor_proto::{
	animation_config, input, output, AnimationConfig, Crop, DrivePath, Error, ErrorCode, EventQueue, Events, Input,
	InputMetadata, InputUpload, Limits, Output, OutputFormat, OutputFormatOptions, Task,
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

	pub fn push(&mut self, path: impl Into<FragmentItem>) -> Fragment<'_> {
		self.path.push(path.into());
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
	pub fn push(&mut self, path: impl Into<FragmentItem>) -> Fragment<'_> {
		self.path.push(path.into());
		Fragment::new(self.path)
	}
}

impl Drop for Fragment<'_> {
	fn drop(&mut self) {
		self.path.pop();
	}
}

pub fn validate_input_upload(
	global: &Arc<Global>,
	mut fragment: Fragment,
	input_upload: Option<&InputUpload>,
) -> Result<(), Error> {
	let input_upload = input_upload.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	validate_drive_path(global, fragment.push("path"), input_upload.path.as_ref(), &["id"])?;

	if input_upload.binary.is_empty() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{fragment}: binary is required"),
		});
	}

	Ok(())
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

	validate_drive_path(
		global,
		fragment.push("path"),
		output.drive_path.as_ref(),
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
	)?;

	if output.formats.is_empty() {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: is required", fragment.push("formats")),
		});
	}

	let mut formats = HashSet::new();
	for (idx, format) in output.formats.iter().enumerate() {
		validate_output_format_options(fragment.push(idx), Some(format), &mut formats)?;
	}

	validate_output_variants_resize(fragment.push("resize"), output.resize.as_ref())?;

	if let Some(animation_config) = output.animation_config.as_ref() {
		validate_output_animation_config(fragment.push("animation_config"), Some(animation_config))?;
	}

	if let Some(crop) = output.crop.as_ref() {
		validate_crop(fragment.push("crop"), Some(crop))?;
	}

	match (output.min_aspect_ratio, output.max_aspect_ratio) {
		(Some(min_ratio), _) if min_ratio <= 0.0 => {
			return Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!("{}: must be greater than or equal to 0", fragment.push("min_ratio")),
			});
		}
		(_, Some(max_ratio)) if max_ratio <= 0.0 => {
			return Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!("{}: must be greater than or equal to 0", fragment.push("max_ratio")),
			});
		}
		(Some(min_ratio), Some(max_ratio)) if min_ratio > max_ratio => {
			return Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!("{}: min_ratio must be less than or equal to max_ratio", fragment),
			});
		}
		_ => {}
	}

	Ok(())
}

pub fn validate_crop(mut fragment: Fragment, crop: Option<&Crop>) -> Result<(), Error> {
	let crop = crop.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	if crop.width == 0 {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: width must be non 0", fragment.push("width")),
		});
	}

	if crop.height == 0 {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: height must be non 0", fragment.push("height")),
		});
	}

	Ok(())
}

pub fn validate_output_animation_config(
	mut fragment: Fragment,
	animation_config: Option<&AnimationConfig>,
) -> Result<(), Error> {
	let animation_config = animation_config.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	if let Some(loop_count) = animation_config.loop_count {
		if loop_count < -1 {
			return Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!(
					"{}: loop_count must be greater than or equal to -1",
					fragment.push("loop_count")
				),
			});
		}
	}

	if let Some(frame_rate) = &animation_config.frame_rate {
		let mut fragment = fragment.push("frame_rate");

		match frame_rate {
			animation_config::FrameRate::DurationMs(duration_ms) => {
				if *duration_ms == 0 {
					return Err(Error {
						code: ErrorCode::InvalidInput as i32,
						message: format!("{}: duration_ms must be non 0", fragment),
					});
				}
			}
			animation_config::FrameRate::DurationsMs(durations_ms) => {
				let mut fragment = fragment.push("durations_ms.values");

				if durations_ms.values.is_empty() {
					return Err(Error {
						code: ErrorCode::InvalidInput as i32,
						message: format!("{fragment}: durations_ms must not be empty"),
					});
				}

				for (idx, duration_ms) in durations_ms.values.iter().enumerate() {
					if *duration_ms == 0 {
						return Err(Error {
							code: ErrorCode::InvalidInput as i32,
							message: format!("{}: duration_ms must be non 0", fragment.push(idx)),
						});
					}
				}
			}
			animation_config::FrameRate::Factor(factor) => {
				if *factor > 0.0 {
					return Err(Error {
						code: ErrorCode::InvalidInput as i32,
						message: format!("{}: factor must be greater than 0", fragment.push("factor")),
					});
				}
			}
		}
	} else {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: frame_rate is required", fragment.push("frame_rate")),
		});
	}

	Ok(())
}

pub fn validate_output_variants_resize(mut fragment: Fragment, resize: Option<&output::Resize>) -> Result<(), Error> {
	let resize = resize.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	let validate_items = |mut fragment: Fragment, items: &[u32]| {
		if items.is_empty() {
			return Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!("{fragment}: is required"),
			});
		}

		for (idx, item) in items.iter().enumerate() {
			if *item == 0 {
				return Err(Error {
					code: ErrorCode::InvalidInput as i32,
					message: format!("{}: must be non 0", fragment.push(idx)),
				});
			}
		}

		Ok(())
	};

	match resize {
		output::Resize::Height(height) => {
			validate_items(fragment.push("height.values"), &height.values)?;
		}
		output::Resize::Width(width) => {
			validate_items(fragment.push("width.values"), &width.values)?;
		}
		output::Resize::Scaling(scaling) => {
			validate_items(fragment.push("scaling.scales"), &scaling.scales)?;
		}
	}

	Ok(())
}

pub fn validate_output_format_options(
	mut fragment: Fragment,
	format: Option<&OutputFormatOptions>,
	formats: &mut HashSet<OutputFormat>,
) -> Result<(), Error> {
	let format = format.ok_or_else(|| Error {
		code: ErrorCode::InvalidInput as i32,
		message: format!("{fragment}: is required"),
	})?;

	if !formats.insert(format.format()) {
		return Err(Error {
			code: ErrorCode::InvalidInput as i32,
			message: format!("{}: format already exists", fragment.push("format")),
		});
	}

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
			validate_drive_path(global, fragment.push("drive_path"), Some(drive_path), &["id"])?;
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
	allowed_vars: &[&str],
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

	validate_template_string(fragment.push("path"), allowed_vars, &drive_path.path)?;

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
