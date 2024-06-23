//! This module contains an auto-deref specialization to help with adding doc
//! comments to sub-types. You can read more about how it works here
//! https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, LinkedList, VecDeque};
use std::hash::Hash;

use super::to_docs_string;

pub trait Settings {
	#[doc(hidden)]
	fn add_docs(
		&self,
		parent_key: &[Cow<'static, str>],
		docs: &mut HashMap<Vec<Cow<'static, str>>, Cow<'static, [Cow<'static, str>]>>,
	) {
		let (_, _) = (parent_key, docs);
	}

	fn docs(&self) -> HashMap<Vec<Cow<'static, str>>, Cow<'static, [Cow<'static, str>]>> {
		let mut docs = HashMap::new();
		self.add_docs(&[], &mut docs);
		docs
	}

	fn to_docs_string(&self) -> Result<String, toml::ser::Error>
	where
		Self: serde::Serialize + Sized,
	{
		to_docs_string(self)
	}
}

#[doc(hidden)]
pub struct Wrapped<T>(pub T);

/// Default implementation for adding docs to a wrapped type.
impl<T> Settings for Wrapped<&T> {}

/// Specialization for adding docs to a type that implements SerdeDocs.
impl<T: Settings> Settings for &Wrapped<&T> {
	fn add_docs(
		&self,
		parent_key: &[Cow<'static, str>],
		docs: &mut HashMap<Vec<Cow<'static, str>>, Cow<'static, [Cow<'static, str>]>>,
	) {
		<T as Settings>::add_docs(self.0, parent_key, docs)
	}
}

/// Specialization for adding docs an array type that implements SerdeDocs.
macro_rules! impl_arr {
    ($($n:literal)+) => {
        $(
            impl<T: Settings> Settings for &Wrapped<&[T; $n]> {
                fn add_docs(&self, parent_key: &[Cow<'static, str>], docs: &mut HashMap<Vec<Cow<'static, str>>, Cow<'static, [Cow<'static, str>]>>) {
                    let mut key = parent_key.to_vec();
                    for (i, item) in self.0.iter().enumerate() {
                        key.push(i.to_string().into());
                        item.add_docs(&key, docs);
                        key.pop();
                    }
                }
            }
        )+
    };
}

impl_arr!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32);

/// Specialization for adding docs to a slice type that implements SerdeDocs.
macro_rules! impl_seq {
    ($($impl_desc:tt)*) => {
        impl $($impl_desc)* {
            fn add_docs(&self, parent_key: &[Cow<'static, str>], docs: &mut HashMap<Vec<Cow<'static, str>>, Cow<'static, [Cow<'static, str>]>>) {
                let mut key = parent_key.to_vec();
                for (i, item) in self.0.iter().enumerate() {
                    key.push(i.to_string().into());
                    item.add_docs(&key, docs);
                    key.pop();
                }
            }
        }
    };
}

impl_seq!(<T: Settings> Settings for &Wrapped<&Vec<T>>);
impl_seq!(<T: Settings> Settings for &Wrapped<&VecDeque<T>>);
impl_seq!(<T: Settings> Settings for &Wrapped<&BinaryHeap<T>>);
impl_seq!(<T: Settings> Settings for &Wrapped<&LinkedList<T>>);
impl_seq!(<T: Settings> Settings for &Wrapped<&BTreeSet<T>>);

/// Specialization for adding docs to a map type that implements SerdeDocs.
macro_rules! impl_map {
    ($($impl_desc:tt)*) => {
        impl $($impl_desc)* {
            fn add_docs(&self, parent_key: &[Cow<'static, str>], docs: &mut HashMap<Vec<Cow<'static, str>>, Cow<'static, [Cow<'static, str>]>>) {
                let mut key = parent_key.to_vec();
                for (k, v) in self.0.iter() {
                    key.push(k.to_string().into());
                    v.add_docs(&key, docs);
                    key.pop();
                }
            }
        }
    };
}

/// Key types for those maps that implement SerdeDocs.
trait Keyable: Hash + PartialOrd + PartialEq + std::fmt::Display {}

macro_rules! impl_keyable {
    ($($t:ty)*) => {
        $(
            impl Keyable for $t {}
        )*
    };
}

impl_keyable!(String &'static str Cow<'static, str> usize u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 bool char);

impl_map!(<K: Keyable, V: Settings> Settings for &Wrapped<&HashMap<K, V>>);
impl_map!(<K: Keyable, V: Settings> Settings for &Wrapped<&BTreeMap<K, V>>);

/// Specialization for adding docs to an option type that implements SerdeDocs.
impl<O: Settings> Settings for &Wrapped<&Option<O>> {
	fn add_docs(
		&self,
		parent_key: &[Cow<'static, str>],
		docs: &mut HashMap<Vec<Cow<'static, str>>, Cow<'static, [Cow<'static, str>]>>,
	) {
		if let Some(inner) = self.0 {
			inner.add_docs(parent_key, docs);
		}
	}
}

/// Specialization for any type that derefs into a type that implements
/// SerdeDocs.
impl<R, V: Settings> Settings for &&Wrapped<&R>
where
	R: std::ops::Deref<Target = V>,
{
	fn add_docs(
		&self,
		parent_key: &[Cow<'static, str>],
		docs: &mut HashMap<Vec<Cow<'static, str>>, Cow<'static, [Cow<'static, str>]>>,
	) {
		(**self.0).add_docs(parent_key, docs);
	}
}

impl Settings for () {}
