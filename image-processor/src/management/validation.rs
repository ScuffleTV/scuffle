use std::sync::Arc;

use scuffle_image_processor_proto::{input_path::InputPath, DrivePath, Error, ErrorCode, Input, InputMetadata, Output, Task};
use url::Url;

use crate::global::Global;


#[derive(Debug, Default, Clone)]
pub struct Fragment {
    path: Vec<&'static str>,
}

impl std::fmt::Display for Fragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.join("."))
    }
}

impl Fragment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, path: &'static str) {
        self.path.push(path);
    }

    pub fn pop(&mut self) {
        self.path.pop();
    }

    pub fn push_clone(&self, path: &'static str) -> Self {
        let mut fragment = self.clone();
        fragment.push(path);
        fragment
    }

    pub fn push_str(&self, path: &str) -> String {
        let mut builder = self.path.join(".");
        if !builder.is_empty() {
            builder.push('.');
        }
        builder.push_str(path);
        builder
    }
}

pub fn validate_task(
    global: &Arc<Global>,
    mut fragment: Fragment,
    task: Option<&Task>,
) -> Result<(), Error> {
    let task = task.ok_or_else(|| Error {
        code: ErrorCode::InvalidInput as i32,
        message: format!("{fragment} is required"),
    })?;

    validate_input(global, fragment.push_clone("input"), task.input.as_ref())?;

    validate_output(global, fragment.push_clone("output"), task.output.as_ref())?;

    Ok(())
}

pub fn validate_output(
    global: &Arc<Global>,
    mut fragment: Fragment,
    output: Option<&Output>,
) -> Result<(), Error> {
    let output = output.ok_or_else(|| Error {
        code: ErrorCode::InvalidInput as i32,
        message: format!("{fragment} is required"),
    })?;

    validate_drive_path(global, fragment.push_clone("path"), output.path.as_ref(), false)?;
    

    Ok(())
}

pub fn validate_input(
    global: &Arc<Global>,
    mut fragment: Fragment,
    input: Option<&Input>
) -> Result<(), Error> {
    let input = input.ok_or_else(|| Error {
        code: ErrorCode::InvalidInput as i32,
        message: format!("{fragment} is required"),
    })?;

    let path = input.path.as_ref().ok_or_else(|| Error {
        code: ErrorCode::InvalidInput as i32,
        message: format!("{} is required", fragment.push_str("path")),
    })?;

    validate_input_path(global, fragment.push_clone("path"), path.input_path.as_ref())?;

    // Metadata is optional
    if let Some(metadata) = &input.metadata {
        validate_input_metadata(global, fragment.push_clone("metadata"), Some(metadata))?;
    }

    Ok(())
}

pub fn validate_input_metadata(
    global: &Arc<Global>,
    mut fragment: Fragment,
    metadata: Option<&InputMetadata>,
) -> Result<(), Error> {
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
                message: format!("{}: static_frame_index must be less than frame_count, {static_frame_index} >= {frame_count}", fragment),
            });
        },
        (Some(_), None) => {
            return Err(Error {
                code: ErrorCode::InvalidInput as i32,
                message: format!("{}: is required when static_frame_index is provided", fragment.push_str("frame_count")),
            });
        },
        _ => {},
    }

    match (metadata.width, metadata.height) {
        (Some(width), Some(height)) => {
            let checks = [
                (width, "width"),
                (height, "height"),
            ];

            for (value, field) in checks {
                if value == 0 {
                    return Err(Error {
                        code: ErrorCode::InvalidInput as i32,
                        message: format!("{}: must be non 0", fragment.push_str(field)),
                    });
                }

                if value > u16::MAX as u32 {
                    return Err(Error {
                        code: ErrorCode::InvalidInput as i32,
                        message: format!("{}: must be less than {}", fragment.push_str(field), u16::MAX),
                    });
                }
            }
        },
        (None, None) => {},
        (Some(_), None) => {
            return Err(Error {
                code: ErrorCode::InvalidInput as i32,
                message: format!("{}: is required when width is provided", fragment.push_str("height")),
            });
        },
        (None, Some(_)) => {
            return Err(Error {
                code: ErrorCode::InvalidInput as i32,
                message: format!("{}: height is required when width is provided", fragment.push_str("width")),
            });
        }
    }

    Ok(())
}

pub fn validate_input_path(
    global: &Arc<Global>,
    mut fragment: Fragment,
    input_path: Option<&InputPath>,
) -> Result<(), Error> {
    let input_path = input_path.ok_or_else(|| Error {
        code: ErrorCode::InvalidInput as i32,
        message: format!("{} is required", fragment.push_str("input_path")),
    })?;

    match input_path {
        InputPath::DrivePath(drive_path) => {
            validate_drive_path(global, fragment.push_clone("input_path.drive_path"), Some(drive_path), true)?;
        },
        InputPath::PublicUrl(url) => {
            validate_public_url(global, fragment.push_clone("input_path.public_url"), url)?;
        },
    }

    Ok(())
}

pub fn validate_drive_path(
    global: &Arc<Global>,
    mut fragment: Fragment,
    drive_path: Option<&DrivePath>,
    is_input: bool,
) -> Result<(), Error> {
    let drive_path = drive_path.ok_or_else(|| Error {
        code: ErrorCode::InvalidInput as i32,
        message: format!("{} is required", fragment),
    })?;

    if global.drive(&drive_path.drive).is_none() {
        return Err(Error {
            code: ErrorCode::InvalidInput as i32,
            message: format!("{}: drive not found", fragment.push_str("drive")),
        });
    }

    const INPUT_PATH_ALLOWED_VARS: &[&str] = &[
        "id",
    ];

    const OUTPUT_PATH_ALLOWED_VARS: &[&str] = &[
        "id",
        "scale",
        "ext",
        "width",
        "format",
        "height",
    ];

    let allowed_vars = if is_input {
        INPUT_PATH_ALLOWED_VARS
    } else {
        OUTPUT_PATH_ALLOWED_VARS
    };

    validate_template_string(allowed_vars, &drive_path.path).map_err(|err| {
        match err {
            strfmt::FmtError::KeyError(key) => Error {
                code: ErrorCode::InvalidInput as i32,
                message: format!("{}: invalid variable '{}' allowed variables {:?}", fragment.push_str("path"), key, allowed_vars),
            },
            strfmt::FmtError::TypeError(_) | strfmt::FmtError::Invalid(_) => Error {
                code: ErrorCode::InvalidInput as i32,
                message: format!("{}: invalid template syntax", fragment.push_str("path")),
            },
        }
    })?;

    Ok(())
}

pub fn validate_public_url(
    global: &Arc<Global>,
    mut fragment: Fragment,
    url: &str,
) -> Result<(), Error> {
    if url.is_empty() {
        return Err(Error {
            code: ErrorCode::InvalidInput as i32,
            message: format!("{} is required", fragment),
        });
    } else if global.public_http_drive().is_none() {
        return Err(Error {
            code: ErrorCode::InvalidInput as i32,
            message: format!("{}: public http drive not found", fragment),
        });
    }

    let url = Url::parse(url).map_err(|e| Error {
        code: ErrorCode::InvalidInput as i32,
        message: format!("{}: {}", fragment, e),
    })?;

    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(Error {
            code: ErrorCode::InvalidInput as i32,
            message: format!("{}: scheme must be http or https", fragment),
        });
    }

    if url.host().is_none() {
        return Err(Error {
            code: ErrorCode::InvalidInput as i32,
            message: format!("{}: host is required", fragment),
        });
    }

    Ok(())
}

fn validate_template_string(
    allowed_vars: &[&str],
    template: &str,
) -> Result<String, strfmt::FmtError> {
    let formatter = |fmt: strfmt::Formatter| {
        let k: &str = fmt.key;
        if !allowed_vars.contains(&k) {
            return Err(strfmt::FmtError::KeyError(k.to_owned()));
        }
        Ok(())
    };
    
    strfmt::strfmt_map(template, formatter)
}
