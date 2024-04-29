use std::{borrow::Cow, collections::HashMap};

use serde_yaml::value::Tag;
use serde_yaml::Value;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "macros")]
pub use scuffle_foundations_macros::{auto_settings, Settings};

#[derive(Debug, Clone)]
pub struct SettingsParser<S> {
    root: serde_yaml::Value,
    _marker: std::marker::PhantomData<S>,
}

enum MergeDirective {
    Unset,
    Replace,
    Merge,
}

impl MergeDirective {
    fn from_tag(tag: &Tag) -> Self {
        if tag == "!replace" {
            Self::Replace
        } else if tag == "!merge" {
            Self::Merge
        } else {
            Self::Unset
        }
    }
}

impl<S> SettingsParser<S> {
    pub fn new(default: &S) -> serde_yaml::Result<Self>
    where
        S: serde::Serialize,
    {
        Ok(Self {
            root: serde_yaml::to_value(default)?,
            _marker: std::marker::PhantomData,
        })
    }

    fn merge(&mut self, mut incoming: serde_yaml::Value) -> serde_yaml::Result<()> {
        self.root.apply_merge()?;
        incoming.apply_merge()?;

        let root = std::mem::take(&mut self.root);
        self.root = self.merge_loop(root, incoming, MergeDirective::Unset);
        Ok(())
    }

    fn merge_loop(
        &self,
        root: serde_yaml::Value,
        incoming: serde_yaml::Value,
        merge: MergeDirective,
    ) -> serde_yaml::Value {
        match (root, incoming) {
            (serde_yaml::Value::Mapping(mut first_map), serde_yaml::Value::Mapping(second_map)) => {
                for (key, value) in second_map {
                    // If the key is tagged we should process it
                    let (key, merge) = match key {
                        serde_yaml::Value::Tagged(tagged) => {
                            (tagged.value, MergeDirective::from_tag(&tagged.tag))
                        }
                        _ => (key, MergeDirective::Unset),
                    };

                    let combined_value = if let Some(existing_value) = first_map.remove(&key) {
                        if matches!(merge, MergeDirective::Replace) {
                            value
                        } else {
                            self.merge_loop(existing_value, value, merge)
                        }
                    } else {
                        value
                    };
                    first_map.insert(key, combined_value);
                }
                serde_yaml::Value::Mapping(first_map)
            }
            (
                serde_yaml::Value::Sequence(mut first_seq),
                serde_yaml::Value::Sequence(second_seq),
            ) => {
                if matches!(merge, MergeDirective::Merge) {
                    first_seq.extend(second_seq);
                } else {
                    first_seq = second_seq;
                }
                serde_yaml::Value::Sequence(first_seq)
            }
            (first, serde_yaml::Value::Tagged(tagged)) => self.handle_tagged(first, *tagged, merge),
            (_, second) => second,
        }
    }

    fn handle_tagged(
        &self,
        first: serde_yaml::Value,
        tagged: serde_yaml::value::TaggedValue,
        merge: MergeDirective,
    ) -> serde_yaml::Value {
        // If the tag is replace it doesn't matter what the first value is
        // we just return the tagged value
        let merge = match (merge, MergeDirective::from_tag(&tagged.tag)) {
            (MergeDirective::Unset, merge) => merge,
            (merge, _) => merge,
        };
        if matches!(merge, MergeDirective::Replace) {
            return tagged.value;
        }
        // If the first value is tagged then we should compare the tags
        // and act accordingly
        if let serde_yaml::Value::Tagged(first_tagged) = first {
            if first_tagged.tag == tagged.tag {
                let value = self.merge_loop(first_tagged.value, tagged.value, merge);
                // Retag the value
                return serde_yaml::Value::Tagged(Box::new(serde_yaml::value::TaggedValue {
                    tag: first_tagged.tag,
                    value,
                }));
            } else {
                return serde_yaml::Value::Tagged(Box::new(tagged));
            }
        }

        // Otherwise we do not merge and retag the value
        let value = self.merge_loop(first, tagged.value, merge);
        if matches!(MergeDirective::from_tag(&tagged.tag), MergeDirective::Unset) {
            serde_yaml::Value::Tagged(Box::new(serde_yaml::value::TaggedValue {
                tag: tagged.tag,
                value,
            }))
        } else {
            value
        }
    }

    pub fn merge_str(&mut self, s: &str) -> serde_yaml::Result<()> {
        let incoming = serde_yaml::from_str(s)?;
        self.merge(incoming)
    }

    pub fn parse(self) -> serde_yaml::Result<S>
    where
        for<'de> S: serde::Deserialize<'de>,
    {
        serde_yaml::from_value(self.root)
    }
}

mod traits;

pub use traits::Wrapped;

pub use traits::Settings;

/// Converts a settings struct to a YAML string including doc comments.
/// If you want to provide doc comments for keys use to_yaml_string_with_docs.
pub fn to_yaml_string<T: serde::Serialize + Settings>(
    settings: &T,
) -> Result<String, serde_yaml::Error> {
    to_yaml_string_with_docs(settings, &settings.docs())
}

type CowStr = Cow<'static, str>;
type DocMap = HashMap<Vec<CowStr>, Cow<'static, [CowStr]>>;

/// Serializes a struct to YAML with documentation comments.
/// Documentation comments are provided in a DocMap.
pub fn to_yaml_string_with_docs<T: serde::Serialize>(
    settings: &T,
    docs: &DocMap,
) -> Result<String, serde_yaml::Error> {
    let data = serde_yaml::to_value(settings)?;
    let mut result = String::new();
    convert_recursive(docs, &mut Vec::new(), &data, &mut result, 0);

    if result.ends_with("\n\n") {
        result.pop();
    } else if !result.ends_with('\n') {
        result.push('\n');
    }

    Ok(result)
}

macro_rules! push_indent {
    ($result: expr, $indent: expr) => {{
        for _ in 0..$indent {
            $result.push(' ');
        }
    }};
}

macro_rules! push_docs {
    ($result: expr, $docs: expr, $stack: expr, $indent: expr) => {{
        $docs
            .get($stack)
            .into_iter()
            .flat_map(|s| s.iter())
            .for_each(|doc| {
                push_indent!($result, $indent);
                $result.push_str("# ");
                $result.push_str(doc);
                push_new_line!($result);
            });
    }};
}

macro_rules! push_key {
    ($result: expr, $key: expr, $indent: expr) => {{
        push_indent!($result, $indent);
        $result.push_str($key);
        $result.push_str(":");
    }};
}

macro_rules! push_new_line {
    ($result: expr) => {{
        if !$result.ends_with('\n') {
            $result.push('\n');
        }
    }};
}

fn convert_recursive(
    docs: &DocMap,
    stack: &mut Vec<CowStr>,
    value: &Value,
    result: &mut String,
    indent: usize,
) {
    // Append doc comments at the current level
    if matches!(value, Value::Mapping(_) | Value::Sequence(_)) {
        stack.push(">".into());
        push_docs!(result, docs, stack, indent);
        stack.pop();
    }

    match value {
        Value::Mapping(map) => {
            for (key, val) in map {
                let key_str = key.as_str().unwrap_or_default();
                stack.push(Cow::from(key_str.to_owned()));

                push_docs!(result, docs, stack, indent);
                push_key!(result, key_str, indent);

                // We dont want to push a new line if the item is a Tagged value
                if matches!(val, Value::Mapping(_) | Value::Sequence(_)) {
                    push_new_line!(result);
                }

                convert_recursive(docs, stack, val, result, indent + 2);

                push_new_line!(result);

                if (val.is_mapping() || val.is_sequence()) && !result.ends_with("\n\n") {
                    result.push('\n');
                }

                stack.pop();
            }

            if map.is_empty() {
                if result.ends_with('\n') {
                    result.pop();
                }
                result.push_str(" {}");
            }
        }
        Value::Sequence(seq) => {
            for (idx, val) in seq.iter().enumerate() {
                stack.push(Cow::from(idx.to_string()));

                push_docs!(result, docs, stack, indent);

                push_indent!(result, indent);
                result.push('-');

                if val.is_sequence() {
                    push_new_line!(result);
                }

                convert_recursive(docs, stack, val, result, indent + 2);

                stack.pop();

                push_new_line!(result);
            }

            if seq.is_empty() {
                if result.ends_with('\n') {
                    result.pop();
                }
                result.push_str(" []");
            }
        }
        Value::Tagged(tagged) => {
            result.push(' ');
            result.push_str(&tagged.tag.to_string());

            if tagged.value.is_mapping() || tagged.value.is_sequence() {
                push_new_line!(result);
            }

            convert_recursive(docs, stack, &tagged.value, result, indent);
        }
        _ => {
            result.push(' ');
            result.push_str(serde_yaml::to_string(value).unwrap_or_default().trim_end());
            // TODO(troy): figure out a way to do sub-docs for scalars so that the format
            //             isnt so janky

            // stack.push(">".into());
            // push_docs!(result, docs, stack, indent);
            // stack.pop();
        }
    }
}
