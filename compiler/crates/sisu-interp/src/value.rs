//! Run-time values.
//!
//! A value is a tree: aggregates own their parts, so value semantics is
//! the Rust semantics of the tree. A reference is a [`Pointer`], a path
//! from a frame's local through fields, payloads and elements; reading or
//! writing through it walks the path, and a pointer into a frame that has
//! returned is detected by the frame's serial number. Moving out of a
//! place leaves [`Value::Moved`] there, so dropping a value whose parts
//! were moved drops only the parts that remain.

use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

/// The `frame` of a pointer into the heap: its `local` is a cell of
/// `Interp::heap` (the value a `Shared` owns), its `serial` the cell's.
pub const HEAP: u32 = u32::MAX;

/// A place reached from a frame's local, or from a heap cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Pointer {
    /// The task whose frame stack holds the frame (06: a task's closure
    /// borrows from the frames of the task that opened its scope).
    pub task: u32,
    /// Index into the task's frame stack, or [`HEAP`].
    pub frame: u32,
    /// The frame's serial number, unique for the run.
    pub serial: u64,
    pub local: u32,
    pub path: Vec<Step>,
    /// The loan this pointer was obtained through, under the ownership
    /// monitor (0: none, a direct access to the frame's local).
    pub tag: u32,
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
    /// The value a `dyn` holds.
    DynValue,
    /// The value a `Mutex` protects.
    MutexValue,
    /// The value in an `Arena` slot.
    Slot(usize),
}

/// A `Mutex`: the value, and whether a guard holds it (and which task's)
/// or left it poisoned.
#[derive(Clone, Debug)]
pub struct MutexVal {
    pub value: Value,
    pub locked: bool,
    pub holder: u32,
    pub poisoned: bool,
}

/// An `Arena`: slots, each with its generation and its value (`Moved`
/// when the slot is free), and the free slots to reuse.
#[derive(Clone, Debug, Default)]
pub struct ArenaVal {
    pub slots: Vec<(i64, Value)>,
    pub free: Vec<usize>,
}

/// A cell of the heap: the value a `Shared` owns, with its owners and weak
/// handles. The value is dropped when `strong` reaches zero; `serial`
/// names the cell for pointers and the monitor.
#[derive(Clone, Debug)]
pub struct HeapCell {
    pub value: Value,
    pub strong: u32,
    pub weak: u32,
    pub serial: u64,
    /// Counted atomically: its construction site may let it reach another
    /// task (the escape analysis of `sisu-borrowck`, Q-2).
    pub atomic: bool,
    /// The task that made it, and whether another task has reached it.
    pub creator: u32,
    pub crossed: bool,
    /// For a capability's cell, the handle type's name (`Fs`, `Cap`).
    pub label: Option<Rc<str>>,
    /// For a `Shared` cell under `--stats shared`, its construction site
    /// (an index of the run's sites).
    pub site: Option<u32>,
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
    U64(u64),
    I128(i128),
    U128(u128),
    Bool(bool),
    Char(char),
    Text(Rc<str>),
    Path(Rc<str>),
    Bytes(Rc<[u8]>),
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
    /// Receives from a channel until it is closed and drained.
    Recv(u32),
    /// A generator: the task that runs its body (`sched.rs`).
    Gen(u32),
    Done,
}

/// A `dyn Trait` value: the value and its type, which a call through the
/// trait dispatches on.
#[derive(Clone, Debug)]
pub struct DynVal {
    pub ty: sisu_sir::Type,
    pub value: Value,
}

/// A closure: its function, the type arguments of the function that made
/// it, and the tuple of captured references.
#[derive(Clone, Debug)]
pub struct ClosureVal {
    pub name: Rc<str>,
    pub tys: Vec<sisu_sir::Type>,
    pub env: Vec<Value>,
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
    /// A `u64`; the other sized integers are `Int`s in their range, but
    /// for the 128-bit ones.
    U64(u64),
    I128(i128),
    U128(u128),
    Float(f64),
    /// An `f32`, a kind of its own so that it prints as the `f32` it is.
    F32(f32),
    Bool(bool),
    Char(char),
    /// A `str`: an immutable view, shared, and under the ownership monitor
    /// the loan of the `String` it views (0 for a literal).
    Str(Rc<str>, u32),
    String(String),
    Path(Rc<str>),
    /// `bytes`: immutable, so its copies share it.
    Bytes(Rc<[u8]>),
    /// Nanoseconds since 1970-01-01T00:00:00Z.
    DateTime(i64),
    Tuple(Vec<Value>),
    Struct(Box<Adt>),
    Variant(Box<Adt>),
    List(Vec<Value>),
    Dict(Box<Dict>),
    Set(Box<Set>),
    Ref(Rc<Pointer>),
    Fn(Rc<str>),
    Closure(Rc<ClosureVal>),
    Dyn(Box<DynVal>),
    /// An owner of a heap cell (`Shared[T]`).
    Shared(u32),
    /// A handle on a heap cell that does not own it (`Weak[T]`).
    Weak(u32),
    /// `Cell[T]`.
    Cell(Box<Value>),
    /// `Mutex[T]`.
    Mutex(Box<MutexVal>),
    /// `MutexGuard[T]`: the lock on the mutex the pointer leads to.
    Guard(Box<Pointer>),
    /// `Arena[T]`.
    Arena(Box<ArenaVal>),
    Iter(Box<IterState>),
    Entry(Box<EntryVal>),
    /// A capability handle (`Fs`, `Clock`, ...) or a `Cap[T]`: a heap cell
    /// that every copy shares and nothing frees. A handle's cell holds its
    /// object as a `dyn` of the object's type; a `Lent` is the handle.
    Cap(u32),
    /// A clock, by its index among the run's clocks (`sched.rs`): the
    /// value of a `HostClock` and of a `TestClock`.
    Clock(u32),
    /// A socket of the host, by its handle: a `HostStream` or a
    /// `HostListener`.
    Socket(u64),
    /// A task scope, a task's handle, a scope's cancellation: indices into
    /// the scheduler's tables.
    Scope(u32),
    Handle(u32),
    Cancellation(u32),
    /// A channel's halves, by channel.
    Sender(u32),
    Receiver(u32),
    /// A moment on a clock: nanoseconds since the epoch of clock `clock`.
    Timer {
        clock: u32,
        at: i128,
    },
}

thread_local! {
    /// The type names of the struct and enum values a thread makes, each
    /// made once and shared by every value of the type.
    static NAMES: std::cell::RefCell<crate::fx::FxSet<Rc<str>>> = Default::default();
    /// The names of the two enums the run time makes most.
    static OPTION: Rc<str> = type_name("Option");
    static RESULT: Rc<str> = type_name("Result");
}

/// The shared name of a type, for its values.
pub fn type_name(ty: &str) -> Rc<str> {
    NAMES.with(|names| {
        if let Some(n) = names.borrow().get(ty) {
            return n.clone();
        }
        let n: Rc<str> = Rc::from(ty);
        names.borrow_mut().insert(n.clone());
        n
    })
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::Str(Rc::from(s), 0)
    }

    pub fn adt(ty: &str, variant: u32, fields: Vec<Value>) -> Box<Adt> {
        Box::new(Adt { ty: type_name(ty), variant, fields, trace: Vec::new() })
    }

    pub fn variant(ty: &str, variant: u32, fields: Vec<Value>) -> Value {
        Value::Variant(Value::adt(ty, variant, fields))
    }

    pub fn some(v: Value) -> Value {
        Value::Variant(Box::new(Adt {
            ty: OPTION.with(Rc::clone),
            variant: 0,
            fields: vec![v],
            trace: Vec::new(),
        }))
    }

    pub fn none() -> Value {
        Value::Variant(Box::new(Adt {
            ty: OPTION.with(Rc::clone),
            variant: 1,
            fields: Vec::new(),
            trace: Vec::new(),
        }))
    }

    pub fn option(v: Option<Value>) -> Value {
        match v {
            Some(v) => Value::some(v),
            None => Value::none(),
        }
    }

    pub fn ok(v: Value) -> Value {
        Value::Variant(Box::new(Adt {
            ty: RESULT.with(Rc::clone),
            variant: 0,
            fields: vec![v],
            trace: Vec::new(),
        }))
    }

    pub fn err(v: Value) -> Value {
        Value::Variant(Box::new(Adt {
            ty: RESULT.with(Rc::clone),
            variant: 1,
            fields: vec![v],
            trace: Vec::new(),
        }))
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
