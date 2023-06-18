use std::{
    any::TypeId,
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fmt::Display,
    ptr::NonNull,
    sync::{Arc, Weak},
};

use crate::{Result, Value};

/// A path to a key.
///
/// The path is represented as a list of [`KeyPathSegment`](KeyPathSegment)s.
///
/// It is iterable and can be created from a string.
///
/// ## Example
///
/// `test.foo[0].bar`
/// is represented as
/// ```
/// [
///     KeyPathSegment::Map { key: Value::String("test") },
///     KeyPathSegment::Seq { index: 0 },
///     KeyPathSegment::Map { key: Value::String("bar") }
/// ]
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct KeyPath {
    segments: Vec<KeyPathSegment>,
}

impl From<&str> for KeyPath {
    fn from(s: &str) -> Self {
        // We need to parse the string for the following cases:
        // - map: foo.bar
        // - seq: foo[0]
        // - sub-seq: foo[0][0]
        // - sub-map: foo.bar.baz

        let mut segments = vec![];
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '[' {
                let mut index = String::new();
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    } else {
                        index.push(c);
                    }
                }

                segments.push(KeyPathSegment::Seq {
                    index: index.parse().unwrap(),
                });
            } else if c == '.' {
                continue;
            } else {
                let mut field = String::new();
                field.push(c);
                while let Some(c) = chars.peek() {
                    if *c == '.' || *c == '[' {
                        break;
                    } else {
                        field.push(*c);
                        chars.next();
                    }
                }

                segments.push(KeyPathSegment::Map {
                    key: Value::String(field),
                });
            }
        }

        Self { segments }
    }
}

impl From<String> for KeyPath {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl IntoIterator for KeyPath {
    type Item = KeyPathSegment;
    type IntoIter = std::vec::IntoIter<KeyPathSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
    }
}

impl<'a> IntoIterator for &'a KeyPath {
    type Item = &'a KeyPathSegment;
    type IntoIter = std::slice::Iter<'a, KeyPathSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.iter()
    }
}

impl KeyPath {
    pub fn iter(&self) -> std::slice::Iter<'_, KeyPathSegment> {
        self.segments.iter()
    }
}

/// A segment of a [`KeyPath`](KeyPath).
///
/// Either a map or a sequence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyPathSegment {
    Map { key: Value },
    Seq { index: usize },
}

impl KeyPathSegment {
    /// Returns true if the segment is a map.
    fn is_map(&self) -> bool {
        matches!(self, Self::Map { .. })
    }

    /// Returns true if the segment is a sequence.
    fn is_seq(&self) -> bool {
        matches!(self, Self::Seq { .. })
    }
}

impl std::fmt::Display for KeyPathSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Map { key } => write!(
                f,
                "{}",
                match key {
                    Value::String(s) => s.to_string(),
                    Value::I8(i) => i.to_string(),
                    Value::I16(i) => i.to_string(),
                    Value::I32(i) => i.to_string(),
                    Value::I64(i) => i.to_string(),
                    Value::U8(i) => i.to_string(),
                    Value::U16(i) => i.to_string(),
                    Value::U32(i) => i.to_string(),
                    Value::U64(i) => i.to_string(),
                    Value::F32(i) => i.to_string(),
                    Value::F64(i) => i.to_string(),
                    Value::Bool(i) => i.to_string(),
                    Value::Char(i) => i.to_string(),
                    Value::Unit => "<unit>".to_string(),
                    Value::Option(_) => "<option>".to_string(),
                    Value::Map(_) => "<map>".to_string(),
                    Value::Seq(_) => "<seq>".to_string(),
                    Value::Bytes(_) => "<bytes>".to_string(),
                    Value::Newtype(_) => "<newtype>".to_string(),
                }
            ),
            Self::Seq { index } => write!(f, "{}", index),
        }
    }
}

impl KeyPath {
    /// Returns the root key path segment.
    pub fn root() -> Self {
        Self { segments: vec![] }
    }

    /// Drops the root key path segment.
    pub fn drop_root(&self) -> Self {
        Self {
            segments: self.segments[1..].to_vec(),
        }
    }

    /// Creates a new key path.
    pub fn new() -> Self {
        Self { segments: vec![] }
    }

    /// Returns a reference to the key path segments.
    pub fn get_inner(&self) -> &[KeyPathSegment] {
        &self.segments
    }

    /// Adds a new [`KeyPathSegment::Map`](KeyPathSegment::Map) to this key path.
    pub fn push_map(&self, key: &Value) -> Self {
        Self {
            segments: {
                let mut segments = self.segments.clone();
                segments.push(KeyPathSegment::Map { key: key.clone() });
                segments
            },
        }
    }

    /// Adds a new [`KeyPathSegment::Seq`](KeyPathSegment::Seq) to this key path.
    pub fn push_seq(&self, index: usize) -> Self {
        Self {
            segments: {
                let mut segments = self.segments.clone();
                segments.push(KeyPathSegment::Seq { index });
                segments
            },
        }
    }

    /// Adds a new [`KeyPathSegment::Map`](KeyPathSegment::Map) containing a [`Value::String`](Value::String) to this key path.
    pub fn push_struct(&self, field: &str) -> Self {
        Self {
            segments: {
                let mut segments = self.segments.clone();
                segments.push(KeyPathSegment::Map {
                    key: Value::String(field.to_string()),
                });
                segments
            },
        }
    }
}

impl Display for KeyPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;

        for segment in &self.segments {
            if segment.is_map() {
                if first {
                    first = false;
                } else {
                    write!(f, ".")?;
                }

                write!(f, "{}", segment)?;
            } else if segment.is_seq() {
                write!(f, "[{}]", segment)?;
            }
        }

        Ok(())
    }
}

thread_local! {
    static MEMO_GRAPHS: RefCell<HashMap<TypeId, (Weak<KeyGraph>, bool)>> = RefCell::new(HashMap::new());
}

enum BuilderMode {
    Owned,
    Ref,
    Building,
}

pub struct KeyGraphBuilder<C: std::any::Any> {
    graph: Arc<KeyGraph>,
    mode: BuilderMode,

    // We use a NonNull<C> here to make sure the compiler enforces that KeyGraphBuilder is not Send, since its a thread_local type
    phantom: std::marker::PhantomData<NonNull<C>>,
}

impl<C: std::any::Any> KeyGraphBuilder<C> {
    pub fn get(&self) -> Option<Arc<KeyGraph>> {
        match self.mode {
            BuilderMode::Building => None,
            BuilderMode::Owned => Some(self.graph.clone()),
            BuilderMode::Ref => Some(Arc::new(KeyGraph::Ref(
                Arc::downgrade(&self.graph),
                std::any::type_name::<C>(),
            ))),
        }
    }

    pub fn build(self, graph: KeyGraph) -> Arc<KeyGraph> {
        if let Some(arc) = self.get() {
            return arc;
        }

        // Safety: If building it means we are the only one
        // who is allowed to mutate the graph. We also know the graph is thread_local so
        // no other thread can access it. This function also takes ownership of the graph builder.
        // Which means that no other thread can access the graph builder, and also this function will not
        // be called again. This means that the graph will not be mutated again.
        // If this function is not called and the builder is dropped, then the Arc<KeyGraph> will be dropped,
        // Therefore the weak pointer will be empty and the graph will be rebuilt when the builder is created again.
        unsafe {
            let graph_ptr = self.graph.as_ref() as *const KeyGraph as *mut KeyGraph;
            let _ = std::mem::replace(&mut *graph_ptr, graph);
        }

        MEMO_GRAPHS.with(|mg| {
            let mut mg = mg.borrow_mut();
            let ty = TypeId::of::<C>();
            if let Some((_, building)) = mg.get_mut(&ty) {
                *building = false;
            }
        });

        self.graph
    }
}

/// A graph of keys.
#[derive(Clone)]
pub enum KeyGraph {
    /// String
    String,
    /// i8
    I8,
    /// i16
    I16,
    /// i32
    I32,
    /// i64
    I64,
    /// u8
    U8,
    /// u16
    U16,
    /// u32
    U32,
    /// u64
    U64,
    /// f32
    F32,
    /// f64
    F64,
    /// bool
    Bool,
    /// ()
    Unit,
    /// char
    Char,
    /// Option
    Option(Arc<KeyGraph>),
    /// Struct
    Struct(BTreeMap<String, Key>),
    /// Map
    Map(Arc<KeyGraph>, Arc<KeyGraph>),
    /// Sequence
    Seq(Arc<KeyGraph>),
    /// Reference
    Ref(Weak<KeyGraph>, &'static str),
}

impl std::fmt::Debug for KeyGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => write!(f, "String"),
            Self::I8 => write!(f, "i8"),
            Self::I16 => write!(f, "i16"),
            Self::I32 => write!(f, "i32"),
            Self::I64 => write!(f, "i64"),
            Self::U8 => write!(f, "u8"),
            Self::U16 => write!(f, "u16"),
            Self::U32 => write!(f, "u32"),
            Self::U64 => write!(f, "u64"),
            Self::F32 => write!(f, "f32"),
            Self::F64 => write!(f, "f64"),
            Self::Bool => write!(f, "bool"),
            Self::Unit => write!(f, "()"),
            Self::Char => write!(f, "char"),
            Self::Option(key) => write!(f, "Option<{:?}>", key),
            Self::Struct(map) => write!(f, "Struct({:?})", map),
            Self::Map(key, value) => write!(f, "Map({:?}, {:?})", key, value),
            Self::Seq(key) => write!(f, "Seq({:?})", key),
            Self::Ref(_, ty) => write!(f, "Ref(&{})", ty),
        }
    }
}

impl KeyGraph {
    pub fn builder<C: std::any::Any>() -> KeyGraphBuilder<C> {
        MEMO_GRAPHS.with(|mg| {
            let mut mg = mg.borrow_mut();

            let ty = TypeId::of::<C>();
            if let Some((graph, building)) = mg.get(&ty) {
                if let Some(graph) = graph.upgrade() {
                    return KeyGraphBuilder {
                        graph,
                        // This check here does 2 things:
                        // 1) Allows for graph memoization (we dont have to rebuild a graph if we already have one for this type) (if building is false)
                        // 2) Allows for recursive types (if building is true, we have not finished building the graph yet but a type has requested it, so we return a reference to the graph)
                        mode: if *building {
                            BuilderMode::Ref
                        } else {
                            BuilderMode::Owned
                        },
                        phantom: std::marker::PhantomData,
                    };
                }
            }

            // If the type isnt in the map or the graph is dead, we need to build it
            // We also need to set building to true so that recursive types work

            // Dummy value does not matter
            // Is overwritten in build
            let graph = Arc::new(KeyGraph::Unit);

            mg.insert(ty, (Arc::downgrade(&graph), true));

            KeyGraphBuilder {
                graph,
                mode: BuilderMode::Building,
                phantom: std::marker::PhantomData,
            }
        })
    }
}

/// Function used to transform a value from whatever it is to the type of the key.
type TransformerFunc = dyn Fn(&KeyPath, Value) -> Result<Value> + Send + Sync;

/// A key
#[derive(Clone)]
pub struct Key {
    graph: Arc<KeyGraph>,
    skip_cli: bool,
    skip_env: bool,
    comment: Option<&'static str>,
    transformer: Option<Arc<TransformerFunc>>,
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Key")
            .field("graph", &self.graph)
            .field("skip_cli", &self.skip_cli)
            .field("skip_env", &self.skip_env)
            .field("comment", &self.comment)
            .finish()
    }
}

impl Key {
    /// Create a new key.
    pub fn new(graph: Arc<KeyGraph>) -> Self {
        Self {
            graph,
            skip_cli: false,
            skip_env: false,
            comment: None,
            transformer: None,
        }
    }

    pub fn with_skip_cli(mut self) -> Self {
        self.skip_cli = true;
        self
    }

    pub fn with_skip_env(mut self) -> Self {
        self.skip_env = true;
        self
    }

    pub fn with_comment(mut self, comment: Option<&'static str>) -> Self {
        self.comment = comment;
        self
    }

    pub fn with_transformer<F>(mut self, transformer: F) -> Self
    where
        F: Fn(&KeyPath, Value) -> Result<Value> + 'static + Send + Sync,
    {
        self.transformer = Some(Arc::new(transformer));
        self
    }

    pub fn transformer(&self) -> Option<&TransformerFunc> {
        self.transformer.as_deref()
    }

    pub fn skip_cli(&self) -> bool {
        self.skip_cli
    }

    pub fn skip_env(&self) -> bool {
        self.skip_env
    }

    pub fn comment(&self) -> Option<&'static str> {
        self.comment
    }

    pub fn graph(&self) -> &KeyGraph {
        &self.graph
    }
}
