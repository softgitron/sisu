//! The boundary between native code and the runtime, shared by both sides:
//! `sisu-codegen` writes what this module describes and the runtime reads
//! it, so the two cannot disagree (docs/architecture.md, "Stage 2").
//!
//! **Calls.** A compiled Sisu function is `fn(ret, arg_1, ..., arg_n) ->
//! u8`: every argument and the result live in memory, and the function
//! gets their addresses; it returns [`OK`] or [`UNWINDING`], the second
//! when a panic is leaving it. A runtime entry point that can panic
//! returns the same status.
//!
//! **Values.** Every type occupies a whole number of 8-byte words (the
//! layout rules are in [`Kind`]), so that generated code moves values as
//! words and the runtime finds a value's parts at fixed offsets.
//!
//! **Types.** The program's concrete types are numbered; generated code
//! names a type by its number, and the runtime reads what the number means
//! from the type table, which the code generator encodes with [`encode`]
//! and the runtime decodes with [`decode`] when the program starts.

/// A function returned normally.
pub const OK: u8 = 0;
/// A panic is leaving the function: the caller continues in its cleanup.
pub const UNWINDING: u8 = 1;

/// The bytes of a word.
pub const WORD: u32 = 8;

/// The symbols of the runtime that generated code refers to.
pub mod sym {
    /// `sisu_rt_start(argc, argv, entry, flags, ret_ty, types, types_len,
    /// fns, fns_len, max_frame) -> i32`: runs the program and gives its
    /// exit status; `max_frame` is the largest native frame of the program,
    /// which the probe reserve of a task's stack must hold.
    pub const START: &str = "sisu_rt_start";
    /// `sisu_rt_frame(name, name_len, file, file_len, line, col)`: a panic
    /// leaves a frame of the program; one line of its stack trace.
    pub const FRAME: &str = "sisu_rt_frame";
    /// `sisu_rt_abort_unwind()`: a panic reached a frame running cleanup.
    pub const ABORT_UNWIND: &str = "sisu_rt_abort_unwind";
    /// `sisu_rt_panic_overflow(op, x, y) -> u8`: integer overflow.
    pub const PANIC_OVERFLOW: &str = "sisu_rt_panic_overflow";
    /// `sisu_rt_panic_index(index, len, from_end) -> u8`: a bad index.
    pub const PANIC_INDEX: &str = "sisu_rt_panic_index";
    /// `sisu_rt_panic_stack() -> u8`: too many nested calls.
    pub const PANIC_STACK: &str = "sisu_rt_panic_stack";
    /// `sisu_rt_panic_unreachable() -> u8`: `unreachable` was reached.
    pub const PANIC_UNREACHABLE: &str = "sisu_rt_panic_unreachable";
    /// `sisu_rt_int_op(op, x, y, out) -> u8`: an integer operation that
    /// may panic for a reason other than overflow (`//`, `%`, `**`, shifts).
    pub const INT_OP: &str = "sisu_rt_int_op";
    /// `sisu_rt_float_op(op, x, y) -> f64`: `%` and `**` on `f64`.
    pub const FLOAT_OP: &str = "sisu_rt_float_op";
    /// `sisu_rt_alloc(bytes) -> ptr`: heap memory for a list or a box.
    pub const ALLOC: &str = "sisu_rt_alloc";
    /// `sisu_rt_drop(value, ty) -> u8`: drops a value of a builtin type.
    pub const DROP: &str = "sisu_rt_drop";
    /// `sisu_rt_copy(dst, src, ty)`: `.copy()`.
    pub const COPY: &str = "sisu_rt_copy";
    /// `sisu_rt_binary(op, dst, a, a_ty, b, b_ty) -> u8`: an operator on
    /// values that are not numbers (equality, order, `in`, `+` on text).
    pub const BINARY: &str = "sisu_rt_binary";
    /// `sisu_rt_intrinsic(desc, ret, args) -> u8`: a call of an intrinsic;
    /// `desc` is a [`super::CallDesc`] and `args` the arguments' addresses.
    pub const INTRINSIC: &str = "sisu_rt_intrinsic";
    /// `sisu_rt_sized(kind, op, x, y, out) -> u8`: an operation of a sized
    /// integer type ([`super::SizedOp`]) on one or two words.
    pub const SIZED: &str = "sisu_rt_sized";
    /// `sisu_rt_wide(kind, op, x, x_ty, y, y_ty, out) -> u8`: an operation
    /// of `i128` or `u128` (by [`super::WIDE_TYPES`]) on the values at `x`
    /// and `y`, of any integer type.
    pub const WIDE: &str = "sisu_rt_wide";
    /// `sisu_rt_table(ty, items, n, out) -> u8`: a `dict` or `set` from a
    /// literal's entries, moved in from `items`.
    pub const TABLE: &str = "sisu_rt_table";
    /// `sisu_rt_dict_slot(dict, key, key_ty, mode, out) -> u8`: the address
    /// of the value a dict holds for a key (`d{k}`), by [`super::SLOT_READ`]
    /// and the other modes.
    pub const DICT_SLOT: &str = "sisu_rt_dict_slot";
    /// A 64-bit counter of the nested calls of the program.
    pub const DEPTH: &str = "SISU_RT_DEPTH";
    /// `sisu_rt_yield()`: a yield point found the budget of the running
    /// task spent; the scheduler may switch tasks.
    pub const YIELD: &str = "sisu_rt_yield";
    /// A 64-bit counter of the yield points the running task may pass
    /// before the scheduler is asked (`sisu_rt_yield` when it reaches 0).
    pub const BUDGET: &str = "SISU_RT_BUDGET";
    /// The running task's stack limits, five words ([`super::STACK_PROBE`]
    /// and the others): what the probe at a function's entry compares the
    /// stack pointer with.
    pub const STACK: &str = "SISU_RT_STACK";
    /// Where the running code calls an intrinsic that may wait, four words
    /// ([`super::SITE_FILE`] and the others): the deadlock report names it.
    pub const SITE: &str = "SISU_RT_SITE";
}

/// The words of [`sym::STACK`]. The probe at a function's entry compares
/// the stack pointer, below the function's frame, with `STACK_PROBE`; when
/// it is lower, a stack pointer at or above `STACK_REAL` only crossed the
/// decommit watermark (set `STACK_DIRTY`, lower `STACK_PROBE` to
/// `STACK_REAL`, go on), and one below it has exhausted the stack: the
/// function sets `STACK_FAILED`, lowers `STACK_PROBE` to `STACK_FLOOR`
/// (the part of the reserve kept for unwinding) and returns
/// [`UNWINDING`] at once, calling nothing; the runtime raises the panic
/// when it next looks at the panics in flight.
pub const STACK_PROBE: u32 = 0;
pub const STACK_REAL: u32 = 8;
pub const STACK_FLOOR: u32 = 16;
pub const STACK_DIRTY: u32 = 24;
pub const STACK_FAILED: u32 = 32;
/// The words of [`sym::STACK`].
pub const STACK_WORDS: usize = 5;

/// The words of [`sym::SITE`]: the address and length of the file's name,
/// the line and column of the call, and flags ([`SITE_PURE`]).
pub const SITE_FILE: u32 = 0;
pub const SITE_FILE_LEN: u32 = 8;
pub const SITE_LINE: u32 = 16;
pub const SITE_COL: u32 = 24;
pub const SITE_FLAGS: u32 = 32;
pub const SITE_WORDS: usize = 5;
/// [`SITE_FLAGS`]: the function making the call is `pure` (a scope opened
/// there reports the lowest-index failure, 06 "Effects").
pub const SITE_PURE: u64 = 1;

/// Nested calls beyond this many panic (the interpreter's limit).
pub const MAX_DEPTH: i64 = 100_000;

/// `flags` of `sisu_rt_start`: `main` returns `T ! E`.
pub const ENTRY_RESULT: u32 = 1;
/// `flags` of `sisu_rt_start`: `main` takes a `System`.
pub const ENTRY_SYSTEM: u32 = 2;

/// `sisu_rt_dict_slot`'s modes: a missing key panics; is inserted (the
/// destination of an assignment); is nothing to drop.
pub const SLOT_READ: u32 = 0;
pub const SLOT_WRITE: u32 = 1;
pub const SLOT_DROP: u32 = 2;

/// The capability handles a `System` holds, in its fields' order
/// (decision 101), and the host's object behind each.
pub const CAPABILITIES: [(&str, &str); 7] = [
    ("Fs", "HostFs"),
    ("Net", "HostNet"),
    ("Clock", "HostClock"),
    ("Rand", "HostRand"),
    ("Env", "HostEnv"),
    ("Proc", "HostProc"),
    ("Stdin", "HostStdin"),
];

/// The integer operations whose failures the runtime reports, and those
/// it computes (`sisu_rt_int_op`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum IntOp {
    Add = 0,
    Sub = 1,
    Mul = 2,
    FloorDiv = 3,
    Mod = 4,
    Pow = 5,
    Shl = 6,
    Shr = 7,
    /// Unary minus: `x` is the operand.
    Neg = 8,
    /// `abs(x)`.
    Abs = 9,
}

impl IntOp {
    pub const ALL: [IntOp; 10] = [
        IntOp::Add,
        IntOp::Sub,
        IntOp::Mul,
        IntOp::FloorDiv,
        IntOp::Mod,
        IntOp::Pow,
        IntOp::Shl,
        IntOp::Shr,
        IntOp::Neg,
        IntOp::Abs,
    ];

    pub fn from_code(c: u32) -> Option<IntOp> {
        IntOp::ALL.get(c as usize).copied()
    }

    /// The operator as Sisu writes it.
    pub fn symbol(self) -> &'static str {
        match self {
            IntOp::Add => "+",
            IntOp::Sub | IntOp::Neg => "-",
            IntOp::Mul => "*",
            IntOp::FloorDiv => "//",
            IntOp::Mod => "%",
            IntOp::Pow => "**",
            IntOp::Shl => "<<",
            IntOp::Shr => ">>",
            IntOp::Abs => "abs",
        }
    }
}

/// The operators `sisu_rt_binary` computes on values that are not numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ValueOp {
    Eq = 0,
    Ne = 1,
    Lt = 2,
    Le = 3,
    Gt = 4,
    Ge = 5,
    /// `a in b`.
    In = 6,
    /// `a + b` on texts: a new `String`.
    Concat = 7,
}

impl ValueOp {
    pub const ALL: [ValueOp; 8] = [
        ValueOp::Eq,
        ValueOp::Ne,
        ValueOp::Lt,
        ValueOp::Le,
        ValueOp::Gt,
        ValueOp::Ge,
        ValueOp::In,
        ValueOp::Concat,
    ];

    pub fn from_code(c: u32) -> Option<ValueOp> {
        ValueOp::ALL.get(c as usize).copied()
    }
}

/// The sized integer types an `i64` word holds, by `sisu_rt_sized`'s
/// number: `u8` to `u64`, then `i8` to `i32`.
pub const SIZED_TYPES: [&str; 7] = ["u8", "u16", "u32", "u64", "i8", "i16", "i32"];

/// The 128-bit integer types, by `sisu_rt_wide`'s number. A value of one
/// is two words, the low one first.
pub const WIDE_TYPES: [&str; 2] = ["i128", "u128"];

macro_rules! sized_ops {
    ($($variant:ident = $name:literal,)*) => {
        /// The operations of the sized integers, by the method names of
        /// their intrinsics (`u8.wrap_add`), and the conversions of `int`
        /// to them (`int.to_u8`, as `ToSized`).
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u32)]
        pub enum SizedOp {
            $($variant,)*
        }

        impl SizedOp {
            pub const ALL: &'static [SizedOp] = &[$(SizedOp::$variant,)*];

            pub fn from_name(n: &str) -> Option<SizedOp> {
                match n {
                    $($name => Some(SizedOp::$variant),)*
                    _ => None,
                }
            }

            pub fn from_code(c: u32) -> Option<SizedOp> {
                SizedOp::ALL.get(c as usize).copied()
            }

            pub fn name(self) -> &'static str {
                match self {
                    $(SizedOp::$variant => $name,)*
                }
            }
        }
    };
}

sized_ops! {
    Add = "add",
    Sub = "sub",
    Mul = "mul",
    FloorDiv = "floordiv",
    Rem = "rem",
    Pow = "pow",
    WrapAdd = "wrap_add",
    WrapSub = "wrap_sub",
    WrapMul = "wrap_mul",
    SatAdd = "sat_add",
    SatSub = "sat_sub",
    SatMul = "sat_mul",
    Shl = "shl",
    Shr = "shr",
    And = "and",
    Or = "or",
    Xor = "xor",
    Not = "not",
    Neg = "neg",
    Cmp = "cmp",
    ToInt = "to_int",
    FromLiteral = "from_literal",
    Widen = "widen",
    FromBits = "from_bits",
    WrappingFrom = "wrapping_from",
    ToSized = "to_sized",
    FromText = "from_text",
}

/// `sisu_rt_float_op`'s operations.
pub const FLOAT_MOD: u32 = 0;
pub const FLOAT_POW: u32 = 1;
pub const FLOAT_FLOORDIV: u32 = 2;

/// One concrete type of the program.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeEntry {
    /// Bytes, a multiple of [`WORD`].
    pub size: u32,
    pub kind: Kind,
}

/// What a type is, and where its parts are. Offsets are in bytes from the
/// start of the value; every part starts on a word.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// `()` and `Never`: no words.
    Unit,
    /// `int`: one word.
    Int,
    /// A sized integer kept in an `i64` word (`u8` to `u32`, `i8` to
    /// `i32`): its name.
    Sized(String),
    /// `u64`: one word, unsigned.
    U64,
    /// `i128`: two words, the low one first, two's complement.
    I128,
    /// `u128`: two words, the low one first.
    U128,
    /// `f64`: one word.
    Float,
    /// `f32`: the low half of one word.
    F32,
    /// `bool`: one word, 0 or 1.
    Bool,
    /// `char`: one word, the code point.
    Char,
    /// `str`: a pointer to UTF-8 bytes and their length.
    Str,
    /// `String`: a pointer to a heap buffer, the length and the capacity,
    /// in bytes.
    String,
    /// `Path`: as `String`.
    Path,
    /// `ref T`: one word, the address of the value; a reference to a
    /// `dyn` is two words, the `dyn`'s own, borrowed.
    Ref(u32),
    /// `list[T]` (and `Array[T, N]`): a pointer to a heap buffer of
    /// elements, the length and the capacity, in elements.
    List(u32),
    /// `Iter[T]`: one word, a runtime object yielding `T`s.
    Iter(u32),
    /// `T?`: a tag word (0 `Some`, 1 `None`) and the payload.
    Option(u32),
    /// `Result[T, E]`: a tag word (0 `Ok`, 1 `Err`) and the payload.
    Result(u32, u32),
    /// A tuple: its elements in order.
    Tuple(Vec<Slot>),
    /// `Box[T]`: one word, a pointer to a heap cell holding the value.
    Box(u32),
    /// A declared struct: its fields in order, and its drop glue (an
    /// index into the program's function table) when it has one.
    Struct { name: String, fields: Vec<Field>, glue: Option<u32> },
    /// A declared enum: a tag word, then the payload of the variant it
    /// holds; an error set's (`error`) starts with the number of its own
    /// type and a word for its return trace, and its tag follows them.
    Enum { name: String, variants: Vec<Variant>, glue: Option<u32>, error: bool },
    /// A union of error sets: a value of one of the member enums (the
    /// numbers of their types), as that enum lays it out.
    ErrSet(Vec<u32>),
    /// A function value: two words, the code and its environment.
    Fn,
    /// A type the runtime only moves and drops by its name.
    Opaque(String),
    /// A constant argument of a type: the `256` of `Array[u32, 256]`. It
    /// has no values.
    Const(i64),
    /// `dict[K, V]`: one word, the address of a table of entries in the
    /// order they were inserted, each the key's words then the value's.
    Dict(u32, u32),
    /// `set[T]`: one word, the address of a table of elements.
    Set(u32),
    /// `Entry[K, V]`: the dict's table and the key.
    Entry(u32, u32),
    /// `dyn Trait`: two words, the number of the type of the value it
    /// holds and the address of a heap cell holding the value, which the
    /// `dyn` owns.
    Dyn,
    /// `Scope`: one word, the scope's number in the scheduler.
    Scope,
    /// `Handle[T]`: one word, the task's number; `T` is its result.
    Handle(u32),
    /// `Cancellation`: one word, the number of the scope it waits for.
    Cancellation,
    /// `Sender[T]` and `Receiver[T]`: one word, the channel's number.
    Sender(u32),
    Receiver(u32),
    /// `Timer`: two words, the clock's number and the moment, in
    /// nanoseconds on that clock.
    Timer,
    /// `Mutex[T]`: [`MUTEX_HEADER`] bytes (whether it is locked, the task
    /// holding it, whether it is poisoned), then the value.
    Mutex(u32),
    /// `MutexGuard[T]`: one word, the address of the value inside the
    /// mutex (its header is just before it), and the function-table index
    /// of `T`'s `invariant` check, which a guard dropped by unwinding runs
    /// (D-077).
    Guard(u32, Option<u32>),
}

/// The bytes of a `Mutex`'s header, before its value.
pub const MUTEX_HEADER: u32 = 24;
/// The words of a `Mutex`'s header, from its start.
pub const MUTEX_LOCKED: u32 = 0;
pub const MUTEX_HOLDER: u32 = 8;
pub const MUTEX_POISONED: u32 = 16;

/// A part of a tuple: its offset and type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub offset: u32,
    pub ty: u32,
}

/// A named part of a struct or a variant.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// `None` for a positional field of a variant.
    pub name: Option<String>,
    pub offset: u32,
    pub ty: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}

/// Where an error set's words are: the number of its type, the trace,
/// the tag, and the payload after them.
pub const ERROR_DECL: u32 = 0;
pub const ERROR_TRACE: u32 = 8;
pub const ERROR_TAG: u32 = 16;
pub const ERROR_PAYLOAD: u32 = 24;

impl Kind {
    /// The offset of an enum's tag word.
    pub fn tag_offset(&self) -> u32 {
        match self {
            Kind::Enum { error: true, .. } | Kind::ErrSet(_) => ERROR_TAG,
            _ => 0,
        }
    }
}

/// What a call of an intrinsic passes (`sisu_rt_intrinsic`), as `u32`
/// words: the intrinsic, the number of arguments and of type arguments,
/// the type of the result, of each argument, and each type argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallDesc {
    pub intrinsic: u32,
    pub ret: u32,
    pub args: Vec<u32>,
    pub targs: Vec<u32>,
}

impl CallDesc {
    pub fn encode(&self) -> Vec<u32> {
        let mut out = vec![self.intrinsic, self.args.len() as u32, self.targs.len() as u32, self.ret];
        out.extend(&self.args);
        out.extend(&self.targs);
        out
    }
}

// ------------------------------------------------------------------ encoding

const K_UNIT: u8 = 0;
const K_INT: u8 = 1;
const K_SIZED: u8 = 2;
const K_U64: u8 = 3;
const K_FLOAT: u8 = 4;
const K_F32: u8 = 5;
const K_BOOL: u8 = 6;
const K_CHAR: u8 = 7;
const K_STR: u8 = 8;
const K_STRING: u8 = 9;
const K_PATH: u8 = 10;
const K_REF: u8 = 11;
const K_LIST: u8 = 12;
const K_ITER: u8 = 13;
const K_OPTION: u8 = 14;
const K_RESULT: u8 = 15;
const K_TUPLE: u8 = 16;
const K_BOX: u8 = 17;
const K_STRUCT: u8 = 18;
const K_ENUM: u8 = 19;
const K_ERRSET: u8 = 20;
const K_FN: u8 = 21;
const K_OPAQUE: u8 = 22;
const K_CONST: u8 = 23;
const K_DICT: u8 = 24;
const K_SET: u8 = 25;
const K_ENTRY: u8 = 26;
const K_I128: u8 = 27;
const K_U128: u8 = 28;
const K_DYN: u8 = 29;
const K_SCOPE: u8 = 30;
const K_HANDLE: u8 = 31;
const K_CANCELLATION: u8 = 32;
const K_SENDER: u8 = 33;
const K_RECEIVER: u8 = 34;
const K_TIMER: u8 = 35;
const K_MUTEX: u8 = 36;
const K_GUARD: u8 = 37;

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, x: u8) {
        self.0.push(x);
    }
    fn u32(&mut self, x: u32) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    fn opt(&mut self, x: Option<u32>) {
        self.u32(x.map(|v| v.wrapping_add(1)).unwrap_or(0));
    }
    fn text(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }
    fn fields(&mut self, fs: &[Field]) {
        self.u32(fs.len() as u32);
        for f in fs {
            match &f.name {
                Some(n) => {
                    self.u8(1);
                    self.text(n);
                }
                None => self.u8(0),
            }
            self.u32(f.offset);
            self.u32(f.ty);
        }
    }
}

/// The type table as bytes, for the program's read-only data.
pub fn encode(table: &[TypeEntry]) -> Vec<u8> {
    let mut w = Writer(Vec::new());
    w.u32(table.len() as u32);
    for t in table {
        w.u32(t.size);
        match &t.kind {
            Kind::Unit => w.u8(K_UNIT),
            Kind::Int => w.u8(K_INT),
            Kind::Sized(n) => {
                w.u8(K_SIZED);
                w.text(n);
            }
            Kind::U64 => w.u8(K_U64),
            Kind::I128 => w.u8(K_I128),
            Kind::Dyn => w.u8(K_DYN),
            Kind::U128 => w.u8(K_U128),
            Kind::Float => w.u8(K_FLOAT),
            Kind::F32 => w.u8(K_F32),
            Kind::Bool => w.u8(K_BOOL),
            Kind::Char => w.u8(K_CHAR),
            Kind::Str => w.u8(K_STR),
            Kind::String => w.u8(K_STRING),
            Kind::Path => w.u8(K_PATH),
            Kind::Ref(t) => {
                w.u8(K_REF);
                w.u32(*t);
            }
            Kind::List(t) => {
                w.u8(K_LIST);
                w.u32(*t);
            }
            Kind::Iter(t) => {
                w.u8(K_ITER);
                w.u32(*t);
            }
            Kind::Option(t) => {
                w.u8(K_OPTION);
                w.u32(*t);
            }
            Kind::Result(a, b) => {
                w.u8(K_RESULT);
                w.u32(*a);
                w.u32(*b);
            }
            Kind::Tuple(slots) => {
                w.u8(K_TUPLE);
                w.u32(slots.len() as u32);
                for s in slots {
                    w.u32(s.offset);
                    w.u32(s.ty);
                }
            }
            Kind::Box(t) => {
                w.u8(K_BOX);
                w.u32(*t);
            }
            Kind::Struct { name, fields, glue } => {
                w.u8(K_STRUCT);
                w.text(name);
                w.opt(*glue);
                w.fields(fields);
            }
            Kind::Enum { name, variants, glue, error } => {
                w.u8(K_ENUM);
                w.text(name);
                w.opt(*glue);
                w.u8(u8::from(*error));
                w.u32(variants.len() as u32);
                for v in variants {
                    w.text(&v.name);
                    w.fields(&v.fields);
                }
            }
            Kind::ErrSet(members) => {
                w.u8(K_ERRSET);
                w.u32(members.len() as u32);
                for m in members {
                    w.u32(*m);
                }
            }
            Kind::Fn => w.u8(K_FN),
            Kind::Opaque(n) => {
                w.u8(K_OPAQUE);
                w.text(n);
            }
            Kind::Const(n) => {
                w.u8(K_CONST);
                w.u32(*n as u64 as u32);
                w.u32((*n as u64 >> 32) as u32);
            }
            Kind::Dict(k, v) => {
                w.u8(K_DICT);
                w.u32(*k);
                w.u32(*v);
            }
            Kind::Set(t) => {
                w.u8(K_SET);
                w.u32(*t);
            }
            Kind::Entry(k, v) => {
                w.u8(K_ENTRY);
                w.u32(*k);
                w.u32(*v);
            }
            Kind::Scope => w.u8(K_SCOPE),
            Kind::Handle(t) => {
                w.u8(K_HANDLE);
                w.u32(*t);
            }
            Kind::Cancellation => w.u8(K_CANCELLATION),
            Kind::Sender(t) => {
                w.u8(K_SENDER);
                w.u32(*t);
            }
            Kind::Receiver(t) => {
                w.u8(K_RECEIVER);
                w.u32(*t);
            }
            Kind::Timer => w.u8(K_TIMER),
            Kind::Mutex(t) => {
                w.u8(K_MUTEX);
                w.u32(*t);
            }
            Kind::Guard(t, check) => {
                w.u8(K_GUARD);
                w.u32(*t);
                w.opt(*check);
            }
        }
    }
    w.0
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u8(&mut self) -> Option<u8> {
        let b = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(b)
    }
    fn u32(&mut self) -> Option<u32> {
        let s = self.bytes.get(self.at..self.at + 4)?;
        self.at += 4;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn opt(&mut self) -> Option<Option<u32>> {
        let v = self.u32()?;
        Some(if v == 0 { None } else { Some(v - 1) })
    }
    fn text(&mut self) -> Option<String> {
        let n = self.u32()? as usize;
        let s = self.bytes.get(self.at..self.at + n)?;
        self.at += n;
        String::from_utf8(s.to_vec()).ok()
    }
    fn fields(&mut self) -> Option<Vec<Field>> {
        let n = self.u32()?;
        let mut out = Vec::new();
        for _ in 0..n {
            let name = if self.u8()? == 1 { Some(self.text()?) } else { None };
            let offset = self.u32()?;
            let ty = self.u32()?;
            out.push(Field { name, offset, ty });
        }
        Some(out)
    }
}

/// The type table [`encode`] wrote; `None` if the bytes are not one.
pub fn decode(bytes: &[u8]) -> Option<Vec<TypeEntry>> {
    let mut r = Reader { bytes, at: 0 };
    let n = r.u32()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let size = r.u32()?;
        let kind = match r.u8()? {
            K_UNIT => Kind::Unit,
            K_INT => Kind::Int,
            K_SIZED => Kind::Sized(r.text()?),
            K_U64 => Kind::U64,
            K_I128 => Kind::I128,
            K_DYN => Kind::Dyn,
            K_U128 => Kind::U128,
            K_FLOAT => Kind::Float,
            K_F32 => Kind::F32,
            K_BOOL => Kind::Bool,
            K_CHAR => Kind::Char,
            K_STR => Kind::Str,
            K_STRING => Kind::String,
            K_PATH => Kind::Path,
            K_REF => Kind::Ref(r.u32()?),
            K_LIST => Kind::List(r.u32()?),
            K_ITER => Kind::Iter(r.u32()?),
            K_OPTION => Kind::Option(r.u32()?),
            K_RESULT => Kind::Result(r.u32()?, r.u32()?),
            K_TUPLE => {
                let k = r.u32()?;
                let mut slots = Vec::new();
                for _ in 0..k {
                    slots.push(Slot { offset: r.u32()?, ty: r.u32()? });
                }
                Kind::Tuple(slots)
            }
            K_BOX => Kind::Box(r.u32()?),
            K_STRUCT => {
                let name = r.text()?;
                let glue = r.opt()?;
                let fields = r.fields()?;
                Kind::Struct { name, fields, glue }
            }
            K_ENUM => {
                let name = r.text()?;
                let glue = r.opt()?;
                let error = r.u8()? == 1;
                let k = r.u32()?;
                let mut variants = Vec::new();
                for _ in 0..k {
                    let vname = r.text()?;
                    let fields = r.fields()?;
                    variants.push(Variant { name: vname, fields });
                }
                Kind::Enum { name, variants, glue, error }
            }
            K_ERRSET => {
                let k = r.u32()?;
                let mut members = Vec::new();
                for _ in 0..k {
                    members.push(r.u32()?);
                }
                Kind::ErrSet(members)
            }
            K_FN => Kind::Fn,
            K_OPAQUE => Kind::Opaque(r.text()?),
            K_CONST => {
                let lo = u64::from(r.u32()?);
                let hi = u64::from(r.u32()?);
                Kind::Const((lo | (hi << 32)) as i64)
            }
            K_DICT => Kind::Dict(r.u32()?, r.u32()?),
            K_SET => Kind::Set(r.u32()?),
            K_ENTRY => Kind::Entry(r.u32()?, r.u32()?),
            K_SCOPE => Kind::Scope,
            K_HANDLE => Kind::Handle(r.u32()?),
            K_CANCELLATION => Kind::Cancellation,
            K_SENDER => Kind::Sender(r.u32()?),
            K_RECEIVER => Kind::Receiver(r.u32()?),
            K_TIMER => Kind::Timer,
            K_MUTEX => Kind::Mutex(r.u32()?),
            K_GUARD => Kind::Guard(r.u32()?, r.opt()?),
            _ => return None,
        };
        out.push(TypeEntry { size, kind });
    }
    Some(out)
}

macro_rules! intrinsics {
    ($($variant:ident = $name:literal,)*) => {
        /// The intrinsics the runtime implements, by the name SIR calls them.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u32)]
        pub enum Intrinsic {
            $($variant,)*
        }

        impl Intrinsic {
            pub const ALL: &'static [Intrinsic] = &[$(Intrinsic::$variant,)*];

            pub fn name(self) -> &'static str {
                match self {
                    $(Intrinsic::$variant => $name,)*
                }
            }

            pub fn from_name(n: &str) -> Option<Intrinsic> {
                match n {
                    $($name => Some(Intrinsic::$variant),)*
                    _ => None,
                }
            }

            pub fn from_id(id: u32) -> Option<Intrinsic> {
                Intrinsic::ALL.get(id as usize).copied()
            }
        }
    };
}

intrinsics! {
    Print = "print",
    PrintErr = "print_err",
    Len = "len",
    Panic = "panic",
    Unreachable = "unreachable",
    Todo = "todo",
    Abs = "abs",
    Min = "min",
    Max = "max",
    Range = "range",
    Enumerate = "enumerate",
    Zip = "zip",
    ZipShortest = "zip_shortest",
    Reversed = "reversed",
    IterableList = "iterable_list",
    Sorted = "sorted",
    Sum = "sum",
    Any = "any",
    All = "all",
    IterRef = "iter.ref",
    IterMut = "iter.mut",
    IterOwn = "iter.own",
    IterNext = "Iter.next",
    IterOf = "Iter.of",
    FmtNew = "fmt.new",
    FmtText = "fmt.text",
    FmtDisplay = "fmt.display",
    FmtDebug = "fmt.debug",
    FmtDigits = "fmt.digits",
    FmtWideDigits = "fmt.wide_digits",
    FmtFloatDigits = "fmt.float_digits",
    FmtFloat32Digits = "fmt.float32_digits",
    FmtFormatter = "fmt.formatter",
    StructuralDisplay = "structural.display",
    StructuralDebug = "structural.debug",
    StructuralEq = "structural.eq",
    StructuralCmp = "structural.cmp",
    StructuralPartialCmp = "structural.partial_cmp",
    StructuralHash = "structural.hash",
    AssertFail = "assert.fail",
    AssertFailCompare = "assert.fail_compare",
    AssertFailParts = "assert.fail_parts",
    ErrorTracePush = "error.trace_push",
    ErrorTrace = "error.trace",
    ErrorIs = "error.is",
    IntCmp = "int.cmp",
    BoolCmp = "bool.cmp",
    CharCmp = "char.cmp",
    IntToF64 = "int.to_f64",
    IntPow = "int.pow",
    F64Cmp = "f64.cmp",
    F64Sqrt = "f64.sqrt",
    F64Floor = "f64.floor",
    F64Ceil = "f64.ceil",
    F64Round = "f64.round",
    F64Abs = "f64.abs",
    F64IsNan = "f64.is_nan",
    F64MulAdd = "f64.mul_add",
    F64WithinUlps = "f64.within_ulps",
    F64TruncateToInt = "f64.truncate_to_int",
    CharFromCode = "char.from_code",
    CharIsDigit = "char.is_digit",
    CharToDigit = "char.to_digit",
    CharIsAlphabetic = "char.is_alphabetic",
    CharIsWhitespace = "char.is_whitespace",
    CharIsUppercase = "char.is_uppercase",
    CharIsLowercase = "char.is_lowercase",
    CharCode = "char.code",
    StrByteLen = "str.byte_len",
    StrByteAt = "str.byte_at",
    StrByteSlice = "str.byte_slice",
    StrBytes = "str.bytes",
    StrCmp = "str.cmp",
    StrChars = "str.chars",
    StrParseFloat = "str.parse_float",
    StrParseF32 = "str.parse_f32",
    StrToLowercase = "str.to_lowercase",
    StrToUppercase = "str.to_uppercase",
    StrToString = "str.to_string",
    StringNew = "String.new",
    StringPush = "String.push",
    StringPushChar = "String.push_char",
    StringAsStr = "String.as_str",
    StringClear = "String.clear",
    StringLen = "String.len",
    ListNew = "list.new",
    ListPush = "list.push",
    ListPop = "list.pop",
    ListLast = "list.last",
    ListFirst = "list.first",
    ListGet = "list.get",
    ListIsEmpty = "list.is_empty",
    ListInsert = "list.insert",
    ListRemove = "list.remove",
    ListExtend = "list.extend",
    ListReverse = "list.reverse",
    ListSwap = "list.swap",
    ListClear = "list.clear",
    ListSlice = "list.slice",
    ListSliceMiddle = "list.slice_middle",
    ListContains = "list.contains",
    ListIter = "list.iter",
    ListJoin = "list.join",
    ListSort = "list.sort",
    ListIndexOf = "list.index_of",
    OptionExpect = "Option.expect",
    OptionUnwrap = "Option.unwrap",
    OptionIsSome = "Option.is_some",
    OptionIsNone = "Option.is_none",
    OptionOr = "Option.or",
    ResultExpect = "Result.expect",
    ResultUnwrap = "Result.unwrap",
    ResultIsOk = "Result.is_ok",
    ResultIsErr = "Result.is_err",
    ResultOk = "Result.ok",
    ResultErr = "Result.err",
    ResultOr = "Result.or",
    F32Cmp = "f32.cmp",
    F32ToF64 = "f32.to_f64",
    F32Sqrt = "f32.sqrt",
    F32Floor = "f32.floor",
    F32Ceil = "f32.ceil",
    F32Round = "f32.round",
    F32Abs = "f32.abs",
    F32IsNan = "f32.is_nan",
    F32MulAdd = "f32.mul_add",
    F32TruncateToInt = "f32.truncate_to_int",
    F32ToBits = "f32.to_bits",
    F32FromBits = "f32.from_bits",
    F32FromLiteral = "f32.from_literal",
    IntToF32 = "int.to_f32",
    F64ToF32 = "f64.to_f32",
    IntWiden = "int.widen",
    HostEnvArgs = "host.env.args",
    HostEnvGet = "host.env.get",
    HostStdinReadLine = "host.stdin.read_line",
    HostRandBits = "host.rand.bits",
    HostFsReadToString = "host.fs.read_to_string",
    HostFsWriteText = "host.fs.write_text",
    HostFsExists = "host.fs.exists",
    HostFsMetadata = "host.fs.metadata",
    HostFsRemove = "host.fs.remove",
    HostFsListDir = "host.fs.list_dir",
    HostFsCreateDir = "host.fs.create_dir",
    HostFsSync = "host.fs.sync",
    ClockNow = "clock.now",
    DateTimeFromUnixNanos = "DateTime.from_unix_nanos",
    DateTimeUnixNanos = "DateTime.unix_nanos",
    DictNew = "dict.new",
    DictGet = "dict.get",
    DictContains = "dict.contains",
    DictIsEmpty = "dict.is_empty",
    DictInsert = "dict.insert",
    DictRemove = "dict.remove",
    DictEntry = "dict.entry",
    DictKeys = "dict.keys",
    DictValues = "dict.values",
    DictItems = "dict.items",
    EntryOrInsert = "Entry.or_insert",
    SetNew = "set.new",
    SetAdd = "set.add",
    SetRemove = "set.remove",
    SetContains = "set.contains",
    SetIsEmpty = "set.is_empty",
    SetIter = "set.iter",
    PathNew = "Path.new",
    PathAsStr = "Path.as_str",
    OpAdd = "op.add",
    OpSub = "op.sub",
    OpMul = "op.mul",
    OpDiv = "op.div",
    OpRem = "op.rem",
    OpNeg = "op.neg",
    ContractFail = "contract.fail",
    ListAsSlice = "list.as_slice",
    SliceToList = "Slice.to_list",
    SliceAt = "Slice.at",
    SliceGet = "Slice.get",
    SliceIsEmpty = "Slice.is_empty",
    SliceSlice = "Slice.slice",
    SliceCut = "Slice.cut",
    SliceIter = "Slice.iter",
    GenericValue = "generic.value",
    ArrayFilled = "Array.filled",
    ArrayToList = "Array.to_list",
    TaskScope = "task.scope",
    TaskRace = "task.race",
    TaskShield = "task.shield",
    TaskYieldNow = "task.yield_now",
    TaskIsCancelled = "task.is_cancelled",
    TaskCheckpoint = "task.checkpoint",
    ScopeSpawn = "Scope.spawn",
    ScopeSpawnWith = "Scope.spawn_with",
    ScopeCancel = "Scope.cancel",
    ScopeIsCancelled = "Scope.is_cancelled",
    ScopeCancelled = "Scope.cancelled",
    HandleJoin = "Handle.join",
    HandleIsFinished = "Handle.is_finished",
    CancellationWait = "Cancellation.wait",
    Channel = "channel",
    SenderSend = "Sender.send",
    SenderTrySend = "Sender.try_send",
    SenderClose = "Sender.close",
    ReceiverRecv = "Receiver.recv",
    ReceiverTryRecv = "Receiver.try_recv",
    ClockAfter = "clock.after",
    TimerWait = "Timer.wait",
    SelectWait = "select.wait",
    LogWrite = "log.write",
    GenNew = "gen.new",
    GenYield = "gen.yield",
    MutexNew = "Mutex.new",
    MutexLock = "Mutex.lock",
    MutexIsPoisoned = "Mutex.is_poisoned",
    MutexIntoInner = "Mutex.into_inner",
}

impl Intrinsic {
    /// Whether a call may make its task wait (or, for a scope, wait at the
    /// scope's end): the code generator records where the call is before
    /// it ([`sym::SITE`]), for the deadlock report.
    pub fn may_wait(self) -> bool {
        matches!(
            self,
            Intrinsic::TaskScope
                | Intrinsic::TaskRace
                | Intrinsic::HandleJoin
                | Intrinsic::CancellationWait
                | Intrinsic::SenderSend
                | Intrinsic::ReceiverRecv
                | Intrinsic::TimerWait
                | Intrinsic::SelectWait
                | Intrinsic::GenYield
                | Intrinsic::MutexLock
                | Intrinsic::IterNext
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind survives the encoding the code generator writes and the
    /// runtime reads.
    #[test]
    fn the_type_table_round_trips() {
        let field =
            |name: Option<&str>, offset: u32, ty: u32| Field { name: name.map(str::to_string), offset, ty };
        let kinds = vec![
            Kind::Unit,
            Kind::Int,
            Kind::Sized("u8".into()),
            Kind::U64,
            Kind::I128,
            Kind::U128,
            Kind::Float,
            Kind::F32,
            Kind::Bool,
            Kind::Char,
            Kind::Str,
            Kind::String,
            Kind::Path,
            Kind::Ref(1),
            Kind::List(2),
            Kind::Iter(3),
            Kind::Option(4),
            Kind::Result(5, 6),
            Kind::Tuple(vec![Slot { offset: 0, ty: 1 }, Slot { offset: 8, ty: 2 }]),
            Kind::Box(7),
            Kind::Struct { name: "Point".into(), fields: vec![field(Some("x"), 0, 1)], glue: Some(3) },
            Kind::Enum {
                name: "Shape".into(),
                variants: vec![
                    Variant { name: "Dot".into(), fields: Vec::new() },
                    Variant { name: "Circle".into(), fields: vec![field(None, 8, 6)] },
                ],
                glue: None,
                error: true,
            },
            Kind::ErrSet(vec![21, 22]),
            Kind::Fn,
            Kind::Opaque("HostFs".into()),
            Kind::Const(256),
            Kind::Dict(1, 2),
            Kind::Set(1),
            Kind::Entry(1, 2),
            Kind::Dyn,
            Kind::Scope,
            Kind::Handle(3),
            Kind::Cancellation,
            Kind::Sender(4),
            Kind::Receiver(4),
            Kind::Timer,
            Kind::Mutex(5),
            Kind::Guard(5, Some(2)),
            Kind::Guard(5, None),
        ];
        let table: Vec<TypeEntry> =
            kinds.into_iter().enumerate().map(|(i, kind)| TypeEntry { size: 8 * i as u32, kind }).collect();
        assert_eq!(decode(&encode(&table)), Some(table));
        assert_eq!(decode(&[1, 0]), None);
    }

    #[test]
    fn intrinsics_and_operations_are_numbered_by_name() {
        for (k, i) in Intrinsic::ALL.iter().enumerate() {
            assert_eq!(Intrinsic::from_name(i.name()), Some(*i));
            assert_eq!(Intrinsic::from_id(k as u32), Some(*i));
        }
        for (k, op) in SizedOp::ALL.iter().enumerate() {
            assert_eq!(SizedOp::from_name(op.name()), Some(*op));
            assert_eq!(SizedOp::from_code(k as u32), Some(*op));
        }
        assert_eq!(Intrinsic::from_name("list.swap"), Some(Intrinsic::ListSwap));
        assert_eq!(SizedOp::from_name("from_text"), Some(SizedOp::FromText));
    }
}
