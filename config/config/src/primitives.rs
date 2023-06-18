use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, SystemTime},
};

use crate::{Config, ConfigError, ConfigErrorType, KeyGraph, KeyPath, Result, Value};

macro_rules! impl_config_primitive {
    ($ty:ty, $kt:expr, |$value:ident, $path:ident| $transform:expr) => {
        impl Config for $ty {
            fn graph() -> Arc<KeyGraph> {
                Arc::new($kt)
            }

            fn transform($path: &KeyPath, $value: Value) -> Result<Value> {
                $transform
            }
        }
    };
    ($ty:ty, $kt:expr) => {
        impl_config_primitive!($ty, $kt, |value| Ok(value));
    };
}

impl_config_primitive!(bool, KeyGraph::Bool, |value, path| {
    match value {
        Value::Bool(_) => Ok(value),
        Value::String(s) => {
            // The bool from str implementation is a bit weird.
            // It only matches "true" or "false", but we want to be more flexible.
            let s = s.to_lowercase();
            let value = match s.as_str() {
                "true" | "yes" | "y" | "1" | "t" | "on" => true,
                "false" | "no" | "n" | "0" | "f" | "off" => false,
                _ => {
                    return Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "{} is not convertable into boolean",
                        s
                    )))
                    .with_path(path.clone()))
                }
            };

            Ok(Value::Bool(value))
        }
        Value::Option(None) => Err(ConfigError::new(ConfigErrorType::ValidationError(
            "missing value".to_string(),
        ))
        .with_path(path.clone())),
        Value::Option(Some(value)) => <Self as Config>::transform(path, *value),
        Value::F32(f) => Ok(Value::Bool(f != 0.0)),
        Value::F64(f) => Ok(Value::Bool(f != 0.0)),
        Value::I8(i) => Ok(Value::Bool(i != 0)),
        Value::I16(i) => Ok(Value::Bool(i != 0)),
        Value::I32(i) => Ok(Value::Bool(i != 0)),
        Value::I64(i) => Ok(Value::Bool(i != 0)),
        Value::U8(i) => Ok(Value::Bool(i != 0)),
        Value::U16(i) => Ok(Value::Bool(i != 0)),
        Value::U32(i) => Ok(Value::Bool(i != 0)),
        Value::U64(i) => Ok(Value::Bool(i != 0)),
        _ => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
            "{:?} is not convertable into boolean",
            value
        )))
        .with_path(path.clone())),
    }
});

macro_rules! out_of_bounds {
    ($ty:ty, $v:ident, $p:ident) => {
        Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
            "{} is not in {}..={} ({})",
            $v,
            <$ty>::MIN,
            <$ty>::MAX,
            stringify!($ty)
        )))
        .with_path($p.clone()))
    };
}

use num_order::NumOrd;
use serde::Deserialize;

macro_rules! bounds_check {
    ($value:ident, $path:ident => $enum:tt, $ty:ty) => {
        match $value {
            Value::String(s) => {
                let value = s.parse::<$ty>().map_err(|_| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "failed to convert {} into {}",
                        s,
                        stringify!($ty)
                    )))
                    .with_path($path.clone())
                })?;
                Ok(Value::$enum(value))
            }
            Value::Option(None) => Err(ConfigError::new(ConfigErrorType::ValidationError(
                "missing value".to_string(),
            ))
            .with_path($path.clone())),
            Value::Option(Some(value)) => <Self as Config>::transform($path, *value),
            Value::Bool(b) => Ok(Value::$enum(if b { 1 as $ty } else { 0 as $ty })),
            Value::F64(f) => {
                if f.num_gt(&<$ty>::MAX) || f.num_lt(&<$ty>::MIN) {
                    out_of_bounds!($ty, f, $path)
                } else {
                    Ok(Value::$enum(f as $ty))
                }
            }
            Value::F32(f) => {
                if f.num_gt(&<$ty>::MAX) || f.num_lt(&<$ty>::MIN) {
                    out_of_bounds!($ty, f, $path)
                } else {
                    Ok(Value::$enum(f as $ty))
                }
            }
            Value::I8(i) => {
                if i.num_gt(&<$ty>::MAX) || i.num_lt(&<$ty>::MIN) {
                    out_of_bounds!($ty, i, $path)
                } else {
                    Ok(Value::$enum(i as $ty))
                }
            }
            Value::I16(i) => {
                if i.num_gt(&<$ty>::MAX) || i.num_lt(&<$ty>::MIN) {
                    out_of_bounds!($ty, i, $path)
                } else {
                    Ok(Value::$enum(i as $ty))
                }
            }
            Value::I32(i) => {
                if i.num_gt(&<$ty>::MAX) || i.num_lt(&<$ty>::MIN) {
                    out_of_bounds!($ty, i, $path)
                } else {
                    Ok(Value::$enum(i as $ty))
                }
            }
            Value::I64(i) => {
                if i.num_gt(&<$ty>::MAX) || i.num_lt(&<$ty>::MIN) {
                    out_of_bounds!($ty, i, $path)
                } else {
                    Ok(Value::$enum(i as $ty))
                }
            }
            Value::U8(i) => {
                if i.num_gt(&<$ty>::MAX) {
                    out_of_bounds!($ty, i, $path)
                } else {
                    Ok(Value::$enum(i as $ty))
                }
            }
            Value::U16(i) => {
                if i.num_gt(&<$ty>::MAX) {
                    out_of_bounds!($ty, i, $path)
                } else {
                    Ok(Value::$enum(i as $ty))
                }
            }
            Value::U32(i) => {
                if i.num_gt(&<$ty>::MAX) {
                    out_of_bounds!($ty, i, $path)
                } else {
                    Ok(Value::$enum(i as $ty))
                }
            }
            Value::U64(i) => {
                if i.num_gt(&<$ty>::MAX) {
                    out_of_bounds!($ty, i, $path)
                } else {
                    Ok(Value::$enum(i as $ty))
                }
            }
            _ => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                "{:?} is not convertable into {}",
                $value,
                stringify!($ty)
            )))
            .with_path($path.clone())),
        }
    };
}

impl_config_primitive!(f32, KeyGraph::F32, |value, path| {
    bounds_check!(value, path => F32, f32)
});
impl_config_primitive!(f64, KeyGraph::F64, |value, path| {
    bounds_check!(value, path => F64, f64)
});
impl_config_primitive!(
    i8,
    KeyGraph::I8,
    |value, path| bounds_check!(value, path => I8, i8)
);
impl_config_primitive!(i16, KeyGraph::I16, |value, path| {
    bounds_check!(value, path => I16, i16)
});
impl_config_primitive!(i32, KeyGraph::I32, |value, path| {
    bounds_check!(value, path => I32, i32)
});
impl_config_primitive!(i64, KeyGraph::I64, |value, path| {
    bounds_check!(value, path => I64, i64)
});
impl_config_primitive!(
    u8,
    KeyGraph::U8,
    |value, path| bounds_check!(value, path => U8, u8)
);

impl_config_primitive!(char, KeyGraph::Char, |value, path| {
    match value {
        Value::Bool(b) => Ok(Value::Char(if b { '1' } else { '0' })),
        Value::String(s) => {
            if s.len() == 1 {
                Ok(Value::Char(s.chars().next().unwrap()))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    s
                )))
                .with_path(path.clone()))
            }
        }
        Value::Option(None) => Err(ConfigError::new(ConfigErrorType::ValidationError(
            "missing value".to_string(),
        ))
        .with_path(path.clone())),
        Value::Option(Some(value)) => <Self as Config>::transform(path, *value),
        Value::F32(f) => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
            "{} is not convertable into char",
            f
        )))),
        Value::F64(f) => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
            "{} is not convertable into char",
            f
        )))),
        Value::I8(i) => {
            if i < 0 {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            } else {
                Ok(Value::Char(i as u8 as char))
            }
        }
        Value::I16(i) => {
            if i < 0 {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            } else if let Some(c) = char::from_u32(i as u32) {
                Ok(Value::Char(c))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            }
        }
        Value::I32(i) => {
            if i < 0 {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            } else if let Some(c) = char::from_u32(i as u32) {
                Ok(Value::Char(c))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            }
        }
        Value::I64(i) => {
            if i < 0 {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            } else if let Some(c) = char::from_u32(i as u32) {
                Ok(Value::Char(c))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            }
        }
        Value::U8(i) => Ok(Value::Char(i as char)),
        Value::U16(i) => {
            if let Some(c) = char::from_u32(i as u32) {
                Ok(Value::Char(c))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            }
        }
        Value::U32(i) => {
            if let Some(c) = char::from_u32(i) {
                Ok(Value::Char(c))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            }
        }
        Value::U64(i) => {
            if i > u32::MAX as u64 {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            } else if let Some(c) = char::from_u32(i as u32) {
                Ok(Value::Char(c))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "{} is not convertable into char",
                    i
                ))))
            }
        }
        Value::Char(c) => Ok(Value::Char(c)),
        _ => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
            "{:?} is not convertable into char",
            value
        )))
        .with_path(path.clone())),
    }
});

impl_config_primitive!(u16, KeyGraph::U16, |value, path| {
    bounds_check!(value, path => U16, u16)
});
impl_config_primitive!(u32, KeyGraph::U32, |value, path| {
    bounds_check!(value, path => U32, u32)
});
impl_config_primitive!(u64, KeyGraph::U64, |value, path| {
    bounds_check!(value, path => U64, u64)
});

impl_config_primitive!(String, KeyGraph::String, |value, path| {
    match value {
        Value::Bool(b) => Ok(Value::String(b.to_string())),
        Value::String(_) => Ok(value),
        Value::Option(None) => Err(ConfigError::new(ConfigErrorType::ValidationError(
            "missing value".to_string(),
        ))
        .with_path(path.clone())),
        Value::Option(Some(value)) => <Self as Config>::transform(path, *value),
        Value::F32(f) => Ok(Value::String(f.to_string())),
        Value::F64(f) => Ok(Value::String(f.to_string())),
        Value::I8(i) => Ok(Value::String(i.to_string())),
        Value::I16(i) => Ok(Value::String(i.to_string())),
        Value::I32(i) => Ok(Value::String(i.to_string())),
        Value::I64(i) => Ok(Value::String(i.to_string())),
        Value::U8(i) => Ok(Value::String(i.to_string())),
        Value::U16(i) => Ok(Value::String(i.to_string())),
        Value::U32(i) => Ok(Value::String(i.to_string())),
        Value::U64(i) => Ok(Value::String(i.to_string())),
        Value::Char(c) => Ok(Value::String(c.to_string())),
        _ => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
            "{:?} is not convertable into string",
            value
        )))
        .with_path(path.clone())),
    }
});

impl_config_primitive!((), KeyGraph::Unit, |_path, _value| Ok(Value::Unit));

impl Config for isize {
    fn graph() -> Arc<KeyGraph> {
        #[cfg(target_pointer_width = "32")]
        return Arc::new(KeyGraph::I32);
        #[cfg(target_pointer_width = "64")]
        return Arc::new(KeyGraph::I64);
    }

    fn transform(path: &KeyPath, value: Value) -> Result<Value> {
        #[cfg(target_pointer_width = "32")]
        return i32::validate(path, value);
        #[cfg(target_pointer_width = "64")]
        return i64::transform(path, value);
    }
}

impl Config for usize {
    fn graph() -> Arc<KeyGraph> {
        #[cfg(target_pointer_width = "32")]
        return Arc::new(KeyGraph::U32);
        #[cfg(target_pointer_width = "64")]
        return Arc::new(KeyGraph::U64);
    }

    fn transform(path: &KeyPath, value: Value) -> Result<Value> {
        #[cfg(target_pointer_width = "32")]
        return u32::validate(value);
        #[cfg(target_pointer_width = "64")]
        return u64::transform(path, value);
    }
}

// Special Type

impl<C: Config> Config for Option<C> {
    fn graph() -> Arc<KeyGraph> {
        let builder = KeyGraph::builder::<Self>();
        if let Some(graph) = builder.get() {
            return graph;
        }

        builder.build(KeyGraph::Option(C::graph()))
    }

    fn transform(path: &KeyPath, value: Value) -> Result<Value> {
        match value {
            Value::Option(None) => Ok(Value::Option(None)),
            _ => {
                let value = C::transform(path, value)?;
                Ok(Value::Option(Some(Box::new(value))))
            }
        }
    }
}

impl Config for Box<str> {
    fn graph() -> Arc<KeyGraph> {
        Arc::new(KeyGraph::String)
    }

    fn transform(path: &KeyPath, value: Value) -> Result<Value> {
        String::transform(path, value)
    }
}

impl<C: Config> Config for PhantomData<C> {
    fn graph() -> Arc<KeyGraph> {
        Arc::new(KeyGraph::Unit)
    }

    fn transform(path: &KeyPath, value: Value) -> Result<Value> {
        <()>::transform(path, value)
    }
}

// Transparent Types

macro_rules! impl_transparent {
    ($ty:tt $(+ $bounds:tt)*) => {
        impl<C: Config $(+ $bounds)*> Config for $ty<C> {
            fn graph() -> Arc<KeyGraph> {
                C::graph()
            }

            fn transform(path: &KeyPath, value: Value) -> Result<Value> {
                C::transform(path, value)
            }
        }
    }
}

impl_transparent!(Box);
impl_transparent!(Cell + Copy);
impl_transparent!(RefCell);
impl_transparent!(Mutex);
impl_transparent!(Rc);
impl_transparent!(Arc);
impl_transparent!(RwLock);

// Sequence Types

pub struct Seq<C: Config>(PhantomData<C>);

fn seq_graph<C: Config>() -> Arc<KeyGraph> {
    let builder = KeyGraph::builder::<Seq<C>>();
    if let Some(graph) = builder.get() {
        return graph;
    }

    builder.build(KeyGraph::Seq(C::graph()))
}

fn seq_transform<C: Config, S: std::any::Any>(path: &KeyPath, value: Value) -> Result<Value> {
    match value {
        Value::Seq(seq) => {
            let mut result = Vec::new();
            for value in seq {
                let value = C::transform(path, value)?;
                result.push(value);
            }

            Ok(Value::Seq(result.into_iter().collect()))
        }
        Value::Option(Some(value)) => seq_transform::<C, S>(path, *value),
        _ => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
            "{:?} is not convertable into {}",
            value,
            std::any::type_name::<S>()
        )))
        .with_path(path.clone())),
    }
}

macro_rules! impl_seq {
    ($ty:path) => {
        impl_seq!($ty|C: Config);
    };
    ($ty:path | $($bounds:tt)*) => {
        impl<$($bounds)*> Config for $ty {
            #[inline(always)]
            fn graph() -> Arc<KeyGraph> {
                seq_graph::<C>()
            }

            #[inline(always)]
            fn transform(path: &KeyPath, value: Value) -> Result<Value> {
                seq_transform::<C, Self>(path, value)
            }
        }
    }
}

impl_seq!(BTreeSet<C> | C: Config + std::cmp::Ord);
impl_seq!(BinaryHeap<C>| C: Config + std::cmp::Ord);
impl_seq!(LinkedList<C>);
impl_seq!(VecDeque<C>);
impl_seq!(Vec<C>);
impl_seq!(Box<[C]>);
impl_seq!(HashSet<C, S> | C: Config + std::hash::Hash + std::cmp::Eq, S: std::hash::BuildHasher + std::default::Default + 'static);

// Map Types

pub struct Map<K: Config, V: Config>(PhantomData<K>, PhantomData<V>);

fn map_graph<K: Config, V: Config>() -> Arc<KeyGraph> {
    let builder = KeyGraph::builder::<Map<K, V>>();
    if let Some(graph) = builder.get() {
        return graph;
    }

    builder.build(KeyGraph::Map(K::graph(), V::graph()))
}

fn map_transform<K: Config, V: Config, S: std::any::Any>(
    path: &KeyPath,
    value: Value,
) -> Result<Value> {
    match value {
        Value::Map(map) => {
            let mut result = BTreeMap::new();
            for (key, value) in map {
                let key = K::transform(path, key)?;
                let value = V::transform(&path.push_map(&key), value)?;
                result.insert(key, value);
            }

            Ok(Value::Map(result))
        }
        Value::Option(Some(value)) => map_transform::<K, V, S>(path, *value),
        _ => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
            "{:?} is not convertable into {}",
            value,
            std::any::type_name::<S>()
        )))
        .with_path(path.clone())),
    }
}

macro_rules! impl_map {
    ($ty:path | $($bounds:tt)*) => {
        impl<$($bounds)*> Config for $ty {
            #[inline(always)]
            fn graph() -> Arc<KeyGraph> {
                map_graph::<K, V>()
            }

            #[inline(always)]
            fn transform(path: &KeyPath, value: Value) -> Result<Value> {
                map_transform::<K, V, Self>(path, value)
            }
        }
    }
}

impl_map!(BTreeMap<K, V> | K: Config + std::cmp::Ord, V: Config);
impl_map!(HashMap<K, V, S> | K: Config + std::hash::Hash + std::cmp::Eq, V: Config, S: std::hash::BuildHasher + std::default::Default + 'static);

// Network Types

macro_rules! impl_network_type {
    ($ty:path) => {
        impl Config for $ty {
            fn graph() -> Arc<KeyGraph> {
                Arc::new(KeyGraph::String)
            }

            fn transform(path: &KeyPath, value: Value) -> Result<Value> {
                match value {
                    Value::String(s) => {
                        let value = s.parse::<$ty>().map_err(|_| {
                            ConfigError::new(ConfigErrorType::ValidationError(format!(
                                "failed to convert {} into {}",
                                s,
                                stringify!($ty)
                            )))
                            .with_path(path.clone())
                        })?;
                        Ok(Value::String(value.to_string()))
                    }
                    Value::Option(Some(value)) => <Self as Config>::transform(path, *value),
                    _ => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "{:?} is not convertable into {}",
                        value,
                        stringify!($ty)
                    )))
                    .with_path(path.clone())),
                }
            }
        }
    };
}

impl_network_type!(IpAddr);
impl_network_type!(Ipv4Addr);
impl_network_type!(Ipv6Addr);
impl_network_type!(SocketAddr);
impl_network_type!(SocketAddrV4);
impl_network_type!(SocketAddrV6);

// Other Miscellaneous Types

impl Config for PathBuf {
    fn graph() -> Arc<KeyGraph> {
        Arc::new(KeyGraph::String)
    }

    fn transform(path: &KeyPath, value: Value) -> Result<Value> {
        String::transform(path, value)
    }
}

impl Config for Duration {
    fn graph() -> Arc<KeyGraph> {
        Arc::new(KeyGraph::String)
    }

    fn transform(path: &KeyPath, value: Value) -> Result<Value> {
        match value {
            Value::String(s) => {
                let value = humantime::parse_duration(&s).map_err(|_| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "failed to convert {} into Duration",
                        s
                    )))
                    .with_path(path.clone())
                })?;

                Ok(serde_value::to_value(value).map_err(|_| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "failed to convert {} into Duration",
                        s
                    )))
                    .with_path(path.clone())
                })?)
            }
            Value::Option(Some(value)) => <Self as Config>::transform(path, *value),
            r => {
                let duration = std::time::Duration::deserialize(r.clone()).map_err(|_| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "{:?} is not convertable into Duration",
                        r
                    )))
                    .with_path(path.clone())
                })?;

                Ok(serde_value::to_value(duration).map_err(|_| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "{:?} is not convertable into Duration",
                        r
                    )))
                    .with_path(path.clone())
                })?)
            }
        }
    }
}

impl Config for SystemTime {
    fn graph() -> Arc<KeyGraph> {
        Arc::new(KeyGraph::String)
    }

    fn transform(path: &KeyPath, value: Value) -> Result<Value> {
        match value {
            Value::String(s) => {
                let value = humantime::parse_rfc3339(&s).map_err(|_| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "failed to convert {} into SystemTime",
                        s
                    )))
                    .with_path(path.clone())
                })?;

                Ok(serde_value::to_value(value).map_err(|_| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "failed to convert {} into SystemTime",
                        s
                    )))
                    .with_path(path.clone())
                })?)
            }
            Value::Option(Some(value)) => <Self as Config>::transform(path, *value),
            r => {
                let system_time = std::time::SystemTime::deserialize(r.clone()).map_err(|_| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "{:?} is not convertable into SystemTime",
                        r
                    )))
                    .with_path(path.clone())
                })?;

                Ok(serde_value::to_value(system_time).map_err(|_| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "{:?} is not convertable into SystemTime",
                        r
                    )))
                    .with_path(path.clone())
                })?)
            }
        }
    }
}

// Compound Types

macro_rules! impl_slice {
    ($size:expr) => {
        impl<C: Config> Config for [C; $size] {
            #[inline(always)]
            fn graph() -> Arc<KeyGraph> {
                seq_graph::<C>()
            }

            fn transform(path: &KeyPath, value: Value) -> Result<Value> {
                match value {
                    Value::Seq(seq) => {
                        if seq.len() != $size {
                            return Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                                "expected {} elements, found {}",
                                $size,
                                seq.len()
                            ))).with_path(path.clone()));
                        }

                        let mut result = Vec::new();
                        for (idx, value) in seq.into_iter().enumerate() {
                            let value = C::transform(&path.push_seq(idx), value)?;
                            result.push(value);
                        }

                        Ok(Value::Seq(result))
                    },
                    Value::Option(Some(value)) => <Self as Config>::transform(path, *value),
                    _ => Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "{:?} is not convertable into [{}; {}]",
                        value,
                        std::any::type_name::<C>(),
                        $size
                    ))).with_path(path.clone())),
                }
            }
        }
    };
    ($size:expr, $($rest:expr),+) => {
        impl_slice!($size);
        impl_slice!($($rest),+);
    };
}

impl_slice!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32
);
