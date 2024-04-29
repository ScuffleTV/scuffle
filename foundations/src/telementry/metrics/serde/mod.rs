//! Serde bridge.

use parking_lot::MappedRwLockReadGuard;
use prometheus_client::{
    encoding::{EncodeLabelSet, EncodeMetric, LabelSetEncoder, MetricEncoder},
    metrics::{
        family::{Family as InnerFamily, MetricConstructor},
        MetricType, TypedMetric,
    },
};
use serde::ser::Serialize;
use std::{fmt, hash::Hash};

mod top;
mod value;

#[derive(Debug)]
enum Error {
    Unexpected(String),
    Fmt(std::fmt::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unexpected(msg) => write!(f, "unexpected error: {}", msg),
            Self::Fmt(_) => write!(f, "formatting error"),
        }
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Unexpected(msg.to_string())
    }
}

/// A wrapper around [`prometheus_client::metrics::family::Family`] which
/// encodes its labels with [`Serialize`] instead of [`Encode`].
///
/// #### Examples
///
/// Basic usage:
///
/// ```rust
/// # use prometheus_client::{
/// #     encoding::text::encode,
/// #     registry::Registry,
/// # };
/// # use prometools::{nonstandard::NonstandardUnsuffixedCounter, serde::Family};
/// # use serde::Serialize;
/// #
/// #[derive(Clone, Eq, Hash, PartialEq, Serialize)]
/// struct Labels {
///     method: Method,
///     host: String,
/// }
///
/// #[derive(Clone, Eq, Hash, PartialEq, Serialize)]
/// enum Method {
///     #[serde(rename = "GET")]
///     Get,
/// }
///
/// let family = <Family<Labels, NonstandardUnsuffixedCounter>>::default();
/// let mut registry = Registry::with_prefix("http");
///
/// registry.register(
///     "incoming_requests",
///     "Number of requests per method and per host",
///     family.clone(),
/// );
///
/// family
///     .get_or_create(&Labels {
///         method: Method::Get,
///         host: "techworkerscoalition.org".to_string(),
///     })
///     .inc();
///
/// let mut serialized = String::new();
///
/// // SAFETY: We know prometheus-client only writes UTF-8 slices.
/// unsafe {
///     encode(&mut serialized.as_mut_vec(), &registry).unwrap();
/// }
///
/// assert_eq!(
///     serialized,
///     concat!(
///         "# HELP http_incoming_requests Number of requests per method and per host.\n",
///         "# TYPE http_incoming_requests counter\n",
///         "http_incoming_requests{method=\"GET\",host=\"techworkerscoalition.org\"} 1\n",
///         "# EOF\n",
///     ),
/// );
/// ```
#[derive(Debug)]
pub struct Family<S, M, C = fn() -> M> {
    inner: InnerFamily<Bridge<S>, M, C>,
}

impl<S, M, C> Family<S, M, C>
where
    S: Clone + Eq + Hash,
{
    pub fn new_with_constructor(constructor: C) -> Self {
        Self {
            inner: InnerFamily::new_with_constructor(constructor),
        }
    }
}

impl<S, M> Default for Family<S, M>
where
    S: Clone + Eq + Hash,
    M: Default,
{
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<S, M, C> Family<S, M, C>
where
    S: Clone + Eq + Hash,
    C: MetricConstructor<M>,
{
    pub fn get_or_create(&self, label_set: &S) -> MappedRwLockReadGuard<M> {
        self.inner.get_or_create(Bridge::from_ref(label_set))
    }
}

impl<S, M, C> EncodeMetric for Family<S, M, C>
where
    S: Clone + Eq + Hash + Serialize,
    M: EncodeMetric + TypedMetric,
    C: MetricConstructor<M>,
{
    fn encode(&self, encoder: MetricEncoder) -> fmt::Result {
        self.inner.encode(encoder)
    }

    fn metric_type(&self) -> MetricType {
        M::TYPE
    }
}

impl<S, M, C> TypedMetric for Family<S, M, C>
where
    M: TypedMetric,
{
    const TYPE: MetricType = <M as TypedMetric>::TYPE;
}

impl<S, M, C> Clone for Family<S, M, C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
#[repr(transparent)]
struct Bridge<S>(S);

impl<S> Bridge<S> {
    fn from_ref(label_set: &S) -> &Self {
        // SAFETY: `Self` is a transparent newtype wrapper.
        unsafe { &*(label_set as *const S as *const Bridge<S>) }
    }
}

impl<S> EncodeLabelSet for Bridge<S>
where
    S: Serialize,
{
    fn encode(&self, encoder: LabelSetEncoder) -> fmt::Result {
        self.0.serialize(top::serializer(encoder)).map_err(|err| {
            tracing::error!("failed to serialize labels: {}", err);
            fmt::Error
        })
    }
}

impl<S> fmt::Debug for Bridge<S>
where
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
