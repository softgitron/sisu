//! Run-time values.
//!
//! A value is a tree: aggregates own their parts, so value semantics is
//! the Rust semantics of the tree. A reference is a [`Pointer`], a path
//! from a frame's local through fields, payloads and elements; reading or
//! writing through it walks the path, and a pointer into a frame that has
//! returned is detected by the frame's serial number. Moving out of a
//! place leaves [`Value::Moved`] there, so dropping a value whose parts
//! were moved drops only the parts that remain.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;

/// A place reached from a frame's local.
#[derive(Clone, Debug, PartialEq)]
pub struct Pointer {
    /// Index into the frame stack.
    pub frame: u32,
    /// The frame's serial number, unique for the run.
    pub serial: u64,
    pub local: u32,
    pub path: Vec<Step>,
}

impl Pointer {
    pub fn join(&self, s: Step) -> Pointer {
        let mut p = self.clone();
        p.path.push(s);
        p
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// A struct field or tuple element.
    Field(u32),
    /// The payload of an enum value of the given variant.
    Payload(u32),
    /// A list element.
    Index(usize),
    /// The key of the dict entry at a position.
    DictKey(usize),
    /// The value of the dict entry at a position.
    DictVal(usize),
    /// A set element at a position.
    SetItem(usize),
}

/// A struct or enum value; `variant` is 0 for a struct. `trace` is the
/// error return trace of an error value (05, "Context and traces").
#[derive(Clone, Debug)]
pub struct Adt {
    pub ty: Rc<str>,
    pub variant: u32,
    pub fields: Vec<Value>,
    pub trace: Vec<Rc<str>>,
}

#[derive(Clone, Debug, Default)]
pub struct Dict {
    pub entries: Vec<(Value, Value)>,
    pub index: HashMap<Key, usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Set {
    pub items: Vec<Value>,
    pub index: HashMap<Key, usize>,
}

/// The hashable form of a value used as a dict key or set element. `str`
/// and `String` share `Text`, so either finds the other.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Unit,
    Int(i64),
    Bool(bool),
    Char(char),
    Text(Rc<str>),
    Path(Rc<str>),
    Tuple(Vec<Key>),
    Adt(Rc<str>, u32, Vec<Key>),
    Handle,
}

impl Dict {
    pub fn position(&self, k: &Key) -> Option<usize> {
        self.index.get(k).copied()
    }

    pub fn insert(&mut self, k: Key, key: Value, value: Value) -> Option<Value> {
        match self.index.get(&k) {
            Some(&i) => Some(std::mem::replace(&mut self.entries[i].1, value)),
            None => {
                self.index.insert(k, self.entries.len());
                self.entries.push((key, value));
                None
            }
        }
    }

    pub fn remove(&mut self, k: &Key) -> Option<(Value, Value)> {
        let i = self.index.remove(k)?;
        let e = self.entries.remove(i);
        for v in self.index.values_mut() {
            if *v > i {
                *v -= 1;
            }
        }
        Some(e)
    }
}

impl Set {
    pub fn insert(&mut self, k: Key, v: Value) -> bool {
        if self.index.contains_key(&k) {
            return false;
        }
        self.index.insert(k, self.items.len());
        self.items.push(v);
        true
    }

    pub fn remove(&mut self, k: &Key) -> bool {
        let Some(i) = self.index.remove(k) else { return false };
        self.items.remove(i);
        for v in self.index.values_mut() {
            if *v > i {
                *v -= 1;
            }
        }
        true
    }
}

/// The file system behind an `Fs` capability.
#[derive(Clone, Debug)]
pub enum FsStore {
    /// The host's file system, through [`crate::Host`].
    Host,
    /// `Fs.in_memory`: files by path, shared by every copy of the handle.
    Memory(Rc<RefCell<BTreeMap<String, String>>>),
}

#[derive(Clone, Debug)]
pub struct FsCap {
    pub store: FsStore,
    pub read_only: bool,
    /// `restrict_to`: paths are relative to this directory.
    pub root: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DictPart {
    Keys,
    Values,
    Items,
}

/// The state of an `Iter`. Adapters hold their source; nothing is
/// computed before `next` asks for it, except where noted.
#[derive(Clone, Debug)]
pub enum IterState {
    Range {
        cur: i64,
        stop: i64,
        step: i64,
    },
    /// Borrows a list: yields references to its elements.
    ListRef {
        base: Pointer,
        idx: usize,
    },
    SetRef {
        base: Pointer,
        idx: usize,
    },
    DictRef {
        base: Pointer,
        idx: usize,
        part: DictPart,
    },
    /// Items computed up front (owned collections, string pieces,
    /// `reversed`).
    Items(VecDeque<Value>),
    Chars {
        text: Rc<str>,
        pos: usize,
    },
    Take {
        inner: Box<IterState>,
        left: i64,
    },
    Skip {
        inner: Box<IterState>,
        n: i64,
    },
    Map {
        inner: Box<IterState>,
        f: Value,
    },
    Filter {
        inner: Box<IterState>,
        f: Value,
    },
    Enumerate {
        inner: Box<IterState>,
        n: i64,
    },
    Zip {
        a: Box<IterState>,
        b: Box<IterState>,
        strict: bool,
    },
    /// Copies the referents of an iterator of references (an `Iterable[T]`
    /// argument whose `T` is not a reference).
    Decay(Box<IterState>),
    Done,
}

#[derive(Clone, Debug)]
pub struct EntryVal {
    pub dict: Pointer,
    pub key: Value,
}

#[derive(Clone, Debug)]
pub enum Value {
    /// An uninitialised or moved-from place.
    Moved,
    Unit,
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    /// A `str`: an immutable view, shared.
    Str(Rc<str>),
    String(String),
    Path(Rc<str>),
    /// Seconds since 1970-01-01T00:00:00Z.
    DateTime(i64),
    Tuple(Vec<Value>),
    Struct(Box<Adt>),
    Variant(Box<Adt>),
    List(Vec<Value>),
    Dict(Box<Dict>),
    Set(Box<Set>),
    Ref(Box<Pointer>),
    Fn(Rc<str>),
    /// A closure: its function and the tuple of captured references.
    Closure(Rc<str>, Rc<Vec<Value>>),
    Iter(Box<IterState>),
    Entry(Box<EntryVal>),
    Fs(Rc<FsCap>),
    Env,
    Clock,
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::Str(Rc::from(s))
    }

    pub fn adt(ty: &str, variant: u32, fields: Vec<Value>) -> Box<Adt> {
        Box::new(Adt { ty: Rc::from(ty), variant, fields, trace: Vec::new() })
    }

    pub fn variant(ty: &str, variant: u32, fields: Vec<Value>) -> Value {
        Value::Variant(Value::adt(ty, variant, fields))
    }

    pub fn some(v: Value) -> Value {
        Value::variant("Option", 0, vec![v])
    }

    pub fn none() -> Value {
        Value::variant("Option", 1, Vec::new())
    }

    pub fn option(v: Option<Value>) -> Value {
        match v {
            Some(v) => Value::some(v),
            None => Value::none(),
        }
    }

    pub fn ok(v: Value) -> Value {
        Value::variant("Result", 0, vec![v])
    }

    pub fn err(v: Value) -> Value {
        Value::variant("Result", 1, vec![v])
    }

    pub fn ordering(o: std::cmp::Ordering) -> Value {
        let i = match o {
            std::cmp::Ordering::Less => 0,
            std::cmp::Ordering::Equal => 1,
            std::cmp::Ordering::Greater => 2,
        };
        Value::variant("Ordering", i, Vec::new())
    }

    pub fn is_moved(&self) -> bool {
        matches!(self, Value::Moved)
    }
}
