//! The runtime's entry points, as the code generator calls them
//! ([`crate::abi::sym`]). Each takes raw addresses that the generated code
//! guarantees, as its documentation in `abi` describes.

use crate::abi::{self, IntOp, Kind, ValueOp};
use crate::mem::{self, List, Text};
use crate::rt::{self, ty};
use crate::sched::{system, system_status};
use crate::value;
use sisu_diag::msg;
use std::ffi::{CStr, c_char};

pub(crate) const EXIT_OK: i32 = 0;
pub(crate) const EXIT_ERROR: i32 = 1;
pub(crate) const EXIT_PANIC: i32 = 101;
pub(crate) const EXIT_ABORT: i32 = 134;

/// The function the driver adds to run one test of a native test
/// executable (decision 460).
pub(crate) const TEST_DISPATCHER: &str = "tests#main";

/// The host stack the program runs on: native frames are larger than the
/// interpreter's, and 100 000 of them must fit (`abi::MAX_DEPTH`). It is
/// the root task's stack, and the system stack runtime code runs on.
pub(crate) const STACK_BYTES: usize = 1 << 30;

/// Of the program's thread's stack, what is kept below its probe limit
/// beyond the reserve: the thread's guard and what the system put there.
const ROOT_MARGIN: usize = 256 * 1024;

/// The program's arguments, after its name.
static ARGS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

pub(crate) fn args() -> &'static [String] {
    ARGS.get().map(Vec::as_slice).unwrap_or(&[])
}

type EntryFn = unsafe extern "C" fn(*mut u8) -> u8;
type SystemEntryFn = unsafe extern "C" fn(*mut u8, *mut u8) -> u8;

/// Runs the program: decodes its type table, runs `entry` on a thread
/// with a large stack, and reports how it ended, as the interpreter does.
///
/// # Safety
/// The arguments are the generated `main`'s: the process's arguments, the
/// entry function, and the program's tables.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
unsafe extern "C" fn sisu_rt_start(
    argc: i32,
    argv: *const *const c_char,
    entry: usize,
    flags: u32,
    ret_ty: u32,
    types: *const u8,
    types_len: u64,
    fns: *const usize,
    fns_len: u64,
    unprobed: u64,
) -> i32 {
    // SAFETY: the generated `main` passes its tables and the C runtime's
    // arguments.
    let (table, fns, args) = unsafe {
        let bytes = std::slice::from_raw_parts(types, types_len as usize);
        let fns = std::slice::from_raw_parts(fns, fns_len as usize).to_vec();
        let mut args = Vec::new();
        for i in 1..argc.max(0) as usize {
            let a = *argv.add(i);
            if !a.is_null() {
                args.push(CStr::from_ptr(a).to_string_lossy().into_owned());
            }
        }
        (crate::abi::decode(bytes), fns, args)
    };
    let Some(table) = table else { rt::internal("the program's type table does not decode") };
    rt::init(table, fns);
    crate::par::set_unprobed(unprobed as usize);
    let _ = ARGS.set(args);
    let run = move || {
        // The root task runs on this thread's stack, whose probe limit is
        // near its bottom; spawned tasks get slots of the pool.
        let unprobed = unprobed as usize;
        let pool = crate::stack::Pool::new(crate::stack::reservation_from_env(), unprobed);
        let low = crate::stack::current_sp().saturating_sub(STACK_BYTES) + ROOT_MARGIN;
        let root_limits = crate::stack::limits(low, pool.reserve, unprobed);
        // `sisu run --native --schedule-seed N` and `--stats` (decision 203).
        let seed =
            std::env::var("SISU_SCHEDULE_SEED").ok().and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
        let keep_stats = std::env::var_os("SISU_STATS").is_some();
        crate::sched::start(seed, crate::sched::Strategy::from_env(), keep_stats, pool, root_limits);
        let size = ty(ret_ty).size.max(8) as usize;
        let mut ret = vec![0u64; size.div_ceil(8)];
        let r = ret.as_mut_ptr() as *mut u8;
        // The interpreter pushes `main`'s frame itself: its entry is not a
        // yield point.
        let status = if flags & abi::ENTRY_SYSTEM != 0 {
            let mut sys = crate::caps::root_system();
            // SAFETY: `entry` is the program's `main`, taking a `System`.
            let f = unsafe { std::mem::transmute::<usize, SystemEntryFn>(entry) };
            // SAFETY: `ret` has room for `main`'s result and `sys` holds a
            // `System`'s seven handles.
            crate::sched::neutral(|| unsafe { f(r, sys.as_mut_ptr() as *mut u8) })
        } else {
            // SAFETY: `entry` is the program's `main`, of this signature.
            let f = unsafe { std::mem::transmute::<usize, EntryFn>(entry) };
            // SAFETY: `ret` has room for `main`'s result.
            crate::sched::neutral(|| unsafe { f(r) })
        };
        // A native test (decision 460): how it ended goes to the file the
        // driver names, apart from what the test wrote to standard error.
        let test_report = std::env::var_os("SISU_TEST_REPORT");
        // A test whose `expect` failed fails at its end with every failure
        // it recorded, before the panic that ended it, if one did, whatever
        // else it did (decisions 388 and 500).
        let expected = rt::take_expected();
        let code = if status != abi::OK {
            match rt::take_panic() {
                Some(p) => {
                    let text = rt::panic_report(&p.message, &p.stack);
                    match &test_report {
                        Some(path) if !expected.is_empty() => {
                            let shown = if rt::hole().is_some() { "" } else { text.as_str() };
                            let _ = std::fs::write(path, format!("failed\n{expected}{shown}"));
                        }
                        Some(_) if rt::hole().is_some() => {}
                        Some(path) => {
                            let _ = std::fs::write(path, format!("failed\n{text}"));
                        }
                        None => rt::stderr(&text),
                    }
                }
                None => rt::internal("a panic left `main` without its message"),
            }
            EXIT_PANIC
        } else if !expected.is_empty()
            && let Some(path) = &test_report
        {
            let _ = std::fs::write(path, format!("failed\n{expected}"));
            EXIT_PANIC
        } else if flags & abi::ENTRY_RESULT != 0 {
            // SAFETY: the result is a `Result` of type `ret_ty`.
            unsafe { main_result(r, ret_ty) }
        } else {
            EXIT_OK
        };
        // The run is over: what the interpreter's host lets go of without
        // dropping it is freed, and no Sisu code runs: the result of a
        // `main` that returned, and the objects of capabilities (decision
        // 223).
        if status == abi::OK {
            // SAFETY: `main` returned its result into `ret`.
            unsafe { value::forget_value(r, ret_ty) };
        }
        crate::caps::release_kept();
        crate::sched::write_trail();
        if let Some(stats) = crate::sched::finish() {
            rt::stderr(&stats_report(&stats));
        }
        // `SISU_RT_ALLOCS`: what the program left allocated (decision 217),
        // and with `peak` the most it held at once (decision 229).
        if let Some(how) = std::env::var_os("SISU_RT_ALLOCS") {
            if how == "peak" {
                let line = rt::render(msg::RUN_PEAK_ALLOCATIONS, &[("count", mem::peak().to_string())]);
                rt::stderr(&format!("{line}\n"));
            }
            let line = rt::render(msg::RUN_LIVE_ALLOCATIONS, &[("count", mem::live().to_string())]);
            rt::stderr(&format!("{line}\n"));
        }
        rt::flush();
        code
    };
    match std::thread::Builder::new().name("sisu-main".into()).stack_size(STACK_BYTES).spawn(run) {
        Ok(h) => h.join().unwrap_or(EXIT_ABORT),
        Err(_) => rt::internal("the program's thread could not start"),
    }
}

/// The `--stats` report of a native run (decision 203): the interpreter's
/// tasks, the most alive at once and the switches, which a seeded run
/// shares with it; the latency in microseconds, the one of the
/// interpreter's two measures native code has (it counts no statements).
fn stats_report(s: &crate::sched::Stats) -> String {
    let pct = |v: &[u64], p: f64| -> u64 {
        if v.is_empty() {
            return 0;
        }
        let mut v = v.to_vec();
        v.sort_unstable();
        let i = ((v.len() as f64 - 1.0) * p).round() as usize;
        v[i.min(v.len() - 1)]
    };
    let lines = [
        rt::render(
            msg::RUN_STATS_TASKS,
            &[
                ("tasks", s.tasks.to_string()),
                ("max", s.max_live.to_string()),
                ("switches", s.switches.to_string()),
            ],
        ),
        rt::render(
            msg::RUN_STATS_LATENCY_NATIVE,
            &[
                ("p50us", pct(&s.latency_micros, 0.5).to_string()),
                ("p99us", pct(&s.latency_micros, 0.99).to_string()),
            ],
        ),
        rt::render(
            msg::RUN_STATS_SHARED,
            &[
                ("local", s.shared_local.to_string()),
                ("crossing", s.shared_crossing.to_string()),
                ("crossed", s.shared_crossed.to_string()),
            ],
        ),
    ];
    let mut out = String::new();
    for l in lines {
        out.push_str(&l);
        out.push('\n');
    }
    out
}

/// The exit status of a `main` that returned `T ! E`: an error is printed
/// with its `Debug` text and its return trace (decision 22).
///
/// # Safety
/// `r` must hold a `Result` of type `t`.
unsafe fn main_result(r: *mut u8, t: u32) -> i32 {
    let Kind::Result(_, err) = ty(t).kind else { return EXIT_OK };
    // SAFETY: the tag and the payload are where a result has them.
    unsafe {
        if mem::word(r, 0) == 0 {
            return EXIT_OK;
        }
        let e = r.add(8);
        let mut out = Vec::new();
        value::debug(&mut out, e, err);
        let mut text =
            rt::render(msg::RUN_ERROR_RETURNED, &[("error", String::from_utf8_lossy(&out).into_owned())]);
        text.push('\n');
        for line in crate::intrinsics::trace_lines(e, err) {
            text.push_str("  ");
            text.push_str(&rt::render(msg::RUN_TRACE_LINE, &[("location", line)]));
            text.push('\n');
        }
        rt::stderr(&text);
    }
    EXIT_ERROR
}

/// A panic leaves a frame of the program: a line of its stack trace.
///
/// # Safety
/// The texts are valid UTF-8 of these lengths.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_frame(
    name: *const u8,
    name_len: u64,
    file: *const u8,
    file_len: u64,
    line: u64,
    col: u64,
) {
    // SAFETY: the generated code passes its static texts.
    let (name, file) = unsafe {
        (mem::Str { ptr: name, len: name_len }.bytes(), mem::Str { ptr: file, len: file_len }.bytes())
    };
    // The dispatcher of a native test executable (decision 460) is no
    // frame of the program's.
    if name == TEST_DISPATCHER.as_bytes() {
        return;
    }
    let file = String::from_utf8_lossy(file);
    let location = if line == 0 { file.into_owned() } else { format!("{file}:{line}:{col}") };
    system(|| {
        let text = rt::render(
            msg::RUN_FRAME,
            &[("function", String::from_utf8_lossy(name).into_owned()), ("location", location)],
        );
        rt::add_frame(text);
    });
}

#[unsafe(no_mangle)]
extern "C" fn sisu_rt_abort_unwind() {
    system(|| rt::abort_unwinding())
}

/// Integer overflow in `x op y` (or `op x` for a unary one).
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_panic_overflow(op: u32, x: i64, y: i64) -> u8 {
    system(|| overflow(op, x, y))
}

fn overflow(op: u32, x: i64, y: i64) -> u8 {
    let expr = match IntOp::from_code(op) {
        Some(IntOp::Neg) => format!("-({x})"),
        Some(IntOp::Abs) => format!("abs({x})"),
        Some(o) => format!("{x} {} {y}", o.symbol()),
        None => format!("{x} ? {y}"),
    };
    rt::raise_msg(msg::PANIC_OVERFLOW, &[("expr", expr)])
}

/// An index outside a list of `len` elements (`from_end` for `xs[^i]`).
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_panic_index(index: i64, len: i64, from_end: u64) -> u8 {
    system(|| {
        if index < 0 && from_end == 0 {
            return rt::raise_msg(msg::PANIC_NEGATIVE_INDEX, &[("index", index.to_string())]);
        }
        let shown = if from_end != 0 { format!("^{index}") } else { index.to_string() };
        rt::raise_msg(msg::PANIC_INDEX_OUT_OF_BOUNDS, &[("len", len.to_string()), ("index", shown)])
    })
}

/// Entering the function named `name` nests more calls than the limit
/// (decision 190): a stack overflow that names it (decision 234), unless a
/// panic's unwinding has room left beyond the limit, when the call goes on
/// ([`abi::OK`]).
///
/// # Safety
/// The name is valid UTF-8 of this length.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_panic_stack(name: *const u8, name_len: u64) -> u8 {
    // SAFETY: the generated code passes its static text.
    let name = unsafe { mem::Str { ptr: name, len: name_len }.bytes() };
    let function = String::from_utf8_lossy(name).into_owned();
    system(|| over_depth(&function))
}

/// The count of nested calls is past [`abi::MAX_DEPTH`]: [`abi::OK`] when a
/// panic unwinds within [`abi::UNWIND_DEPTH`] more, else the overflow,
/// raised.
pub(crate) fn over_depth(function: &str) -> u8 {
    let d = crate::tls::depth().load(std::sync::atomic::Ordering::Relaxed);
    if d <= abi::MAX_DEPTH + abi::UNWIND_DEPTH && rt::unwinding() {
        return abi::OK;
    }
    rt::raise_msg(msg::PANIC_STACK_OVERFLOW, &[("function", function.to_string())])
}

#[unsafe(no_mangle)]
extern "C" fn sisu_rt_panic_unreachable() -> u8 {
    system(|| rt::raise_msg(msg::PANIC_UNREACHABLE_PLAIN, &[]))
}

/// `x op y` for the operations that can fail other than by overflow, with
/// the interpreter's rules (`//` floors, `%` takes the divisor's sign).
///
/// # Safety
/// `out` must be a word.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_int_op(op: u32, x: i64, y: i64, out: *mut i64) -> u8 {
    match int_op(op, x, y) {
        Ok(v) => {
            // SAFETY: `out` is a word.
            unsafe { out.write(v) };
            abi::OK
        }
        Err(status) => status,
    }
}

pub(crate) fn int_op(op: u32, x: i64, y: i64) -> Result<i64, u8> {
    let Some(o) = IntOp::from_code(op) else { rt::internal("an unknown integer operation") };
    let overflow = || system(|| overflow(op, x, y));
    match o {
        IntOp::Add => x.checked_add(y).ok_or_else(overflow),
        IntOp::Sub => x.checked_sub(y).ok_or_else(overflow),
        IntOp::Mul => x.checked_mul(y).ok_or_else(overflow),
        IntOp::Neg => x.checked_neg().ok_or_else(overflow),
        IntOp::Abs => x.checked_abs().ok_or_else(overflow),
        IntOp::FloorDiv | IntOp::Mod if y == 0 => {
            Err(system(|| rt::raise_msg(msg::PANIC_DIVIDE_BY_ZERO, &[])))
        }
        IntOp::FloorDiv => {
            let q = x.checked_div(y).ok_or_else(overflow)?;
            Ok(if x % y != 0 && ((x < 0) != (y < 0)) { q - 1 } else { q })
        }
        IntOp::Mod => {
            let r = x.checked_rem(y).unwrap_or(0);
            Ok(if r != 0 && ((r < 0) != (y < 0)) { r + y } else { r })
        }
        IntOp::Pow => {
            if y < 0 {
                return Err(system(|| {
                    rt::raise_msg(msg::PANIC_NEGATIVE_EXPONENT, &[("expr", format!("{x} ** {y}"))])
                }));
            }
            match u32::try_from(y) {
                Ok(e) => x.checked_pow(e).ok_or_else(overflow),
                Err(_) => match x {
                    0 | 1 => Ok(x),
                    -1 => Ok(if y % 2 == 0 { 1 } else { -1 }),
                    _ => Err(overflow()),
                },
            }
        }
        IntOp::Shl | IntOp::Shr => {
            if !(0..64).contains(&y) {
                return Err(system(|| {
                    rt::raise_msg(msg::PANIC_SHIFT_OUT_OF_RANGE, &[("amount", y.to_string())])
                }));
            }
            Ok(if o == IntOp::Shl { x << y } else { x >> y })
        }
    }
}

/// `%`, `**` and `//` on `f64` (decision 18 for `%`).
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_float_op(op: u32, x: f64, y: f64) -> f64 {
    match op {
        abi::FLOAT_MOD => float_mod(x, y),
        abi::FLOAT_POW => x.powf(y),
        _ => (x / y).floor(),
    }
}

pub(crate) fn float_mod(x: f64, y: f64) -> f64 {
    let r = x % y;
    if r == 0.0 {
        0.0f64.copysign(y)
    } else if (r < 0.0) != (y < 0.0) {
        r + y
    } else {
        r
    }
}

#[unsafe(no_mangle)]
extern "C" fn sisu_rt_alloc(bytes: u64) -> *mut u8 {
    let p = system(|| mem::alloc(bytes as usize) as usize);
    p as *mut u8
}

/// The environment of a task's closure (or a generator's), `bytes` long,
/// of type `t`: a block whose word before the environment holds the type,
/// which a generator abandoned before it starts needs to forget what the
/// environment holds ([`release_task_env`] frees it).
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_task_env(bytes: u64, t: u32) -> *mut u8 {
    let p = system(|| {
        let p = mem::alloc(bytes as usize + 8);
        // SAFETY: the block has room for the type's word and the value.
        unsafe {
            mem::set_word(p, 0, u64::from(t));
            p.add(8) as usize
        }
    });
    p as *mut u8
}

/// Frees an environment [`sisu_rt_task_env`] made; with `drop`, the
/// values it holds are dropped first (a generator closed before it
/// started, whose closure never took them, decision 236), and the status
/// is their drop's.
pub(crate) fn release_task_env(env: usize, drop: bool) -> u8 {
    if env == 0 {
        return abi::OK;
    }
    let p = env as *mut u8;
    // SAFETY: an environment is preceded by the word of its type, in the
    // block `sisu_rt_task_env` made.
    unsafe {
        let block = p.sub(8);
        let status = if drop { value::drop_value(p, mem::word(block, 0) as u32) } else { abi::OK };
        mem::release(block);
        status
    }
}

/// A copy of the value at `p`, of type `t`, was made: each capability
/// handle it holds counts one more (decision 237).
///
/// # Safety
/// `p` must hold a value of type `t`.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_retain(p: *mut u8, t: u32) {
    // SAFETY: as the caller guarantees.
    unsafe { value::retain_value(p, t) }
}

/// Frees a block of `sisu_rt_alloc` whose value is gone (decision 216).
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_free(p: *mut u8) {
    let p = p as usize;
    system(|| mem::release(p as *mut u8));
}

/// A field was moved out of the error value at `p`: the value is never
/// dropped whole, so it lets go of its return trace now (decision 215).
///
/// # Safety
/// `p` must hold an error value.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_trace_release(p: *mut u8) {
    let p = p as usize;
    // SAFETY: as the caller guarantees.
    system(|| unsafe {
        let p = p as *mut u8;
        crate::intrinsics::release_trace(mem::word(p, abi::ERROR_TRACE));
        mem::set_word(p, abi::ERROR_TRACE, 0);
    });
}

/// Drops the value at `p` of type `t`: by its glue (Sisu code) on the
/// task's stack, and structurally on the system stack (`value::drop_value`).
///
/// # Safety
/// `p` must hold a value of type `t`, dead afterwards.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_drop(p: *mut u8, t: u32) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe { value::drop_value(p, t) }
}

/// `.copy()`.
///
/// # Safety
/// `src` must hold a value of type `t` and `dst` have room for one.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_copy(dst: *mut u8, src: *mut u8, t: u32) {
    // SAFETY: as the caller guarantees.
    system(|| unsafe { value::copy_value(dst, src, t) })
}

/// An operator on values that are not numbers: the comparisons write a
/// `bool` word at `dst`, `+` on texts a `String`.
///
/// # Safety
/// `a` and `b` must hold values of their types, and `dst` have room for
/// the result.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_binary(op: u32, dst: *mut u8, a: *mut u8, ta: u32, b: *mut u8, tb: u32) -> u8 {
    // SAFETY: as the caller guarantees.
    system(|| unsafe { binary(op, dst, a, ta, b, tb) })
}

/// # Safety
/// As `sisu_rt_binary`.
unsafe fn binary(op: u32, dst: *mut u8, a: *mut u8, ta: u32, b: *mut u8, tb: u32) -> u8 {
    let Some(o) = ValueOp::from_code(op) else { rt::internal("an unknown operator") };
    // SAFETY: as the caller guarantees.
    unsafe {
        let r = match o {
            ValueOp::Eq => value::equal(a, ta, b, tb),
            ValueOp::Ne => !value::equal(a, ta, b, tb),
            ValueOp::Lt | ValueOp::Le | ValueOp::Gt | ValueOp::Ge => match value::compare(a, ta, b, tb) {
                None => false,
                Some(c) => match o {
                    ValueOp::Lt => c.is_lt(),
                    ValueOp::Le => c.is_le(),
                    ValueOp::Gt => c.is_gt(),
                    _ => c.is_ge(),
                },
            },
            ValueOp::In => crate::intrinsics::contains(b, tb, a, ta),
            ValueOp::Concat => {
                let x = value::text(a, ta).unwrap_or(&[]);
                let y = value::text(b, tb).unwrap_or(&[]);
                let mut t = Text::from_bytes(x);
                t.push(y);
                (dst as *mut Text).write(t);
                return abi::OK;
            }
        };
        mem::set_word(dst, 0, u64::from(r));
    }
    abi::OK
}

/// The block at `p` (null for none) grown, or made, to `bytes`, keeping
/// what it held: a text's buffer that the runtime in Sisu builds (decisions
/// 265 and 350).
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_grow(p: *mut u8, bytes: i64) -> *mut u8 {
    let p = p as usize;
    system(|| mem::grow(p as *mut u8, bytes.max(0) as usize) as usize) as *mut u8
}

/// Looks a key up in the `dict` or `set` at `t`, without the intrinsic's
/// descriptor (decision 265): for `dict.get` (`value` 1) the address of
/// the key's value, for `contains` 1, when the key is there; else 0.
///
/// # Safety
/// `t` must hold a dict or a set, and `key` a value of type `kt`.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_table_find(t: *mut u8, key: *mut u8, kt: u32, value: u32) -> u64 {
    let (t, key) = (t as usize, key as usize);
    // SAFETY: as the caller guarantees.
    system(|| unsafe {
        let table = crate::dict::table(t as *mut u8);
        match table.position(key as *mut u8, kt) {
            Some(i) if value != 0 => table.val_at(i) as u64,
            Some(_) => 1,
            None => 0,
        }
    })
}

/// Room for one more element of `size` bytes in the list at `l`: a push
/// compiled inline that finds no room (decision 265).
///
/// # Safety
/// `l` must hold a list of elements of `size` bytes.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_list_reserve(l: *mut u8, size: u64) {
    let l = l as usize;
    // SAFETY: as the caller guarantees.
    system(|| unsafe { mem::at::<List>(l as *mut u8).reserve(1, size as u32) })
}

/// A call of an intrinsic: `desc` says which and the types of its
/// result, arguments and type arguments ([`abi::CallDesc`]); `args` holds
/// the arguments' addresses.
///
/// # Safety
/// The descriptor and the addresses are the generated code's.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_intrinsic(desc: *const u32, ret: *mut u8, args: *const *mut u8) -> u8 {
    // SAFETY: as the caller guarantees.
    let (i, c) = unsafe { crate::intrinsics::decode(desc, ret, args) };
    // std.par's bodies are Sisu code, run on the workers or on this stack
    // (decision 333).
    if i == abi::Intrinsic::ParMap {
        // SAFETY: as the caller guarantees.
        return unsafe { crate::par::map(&c) };
    }
    if crate::tasks::handles(i) {
        // SAFETY: as the caller guarantees.
        return unsafe { crate::tasks::call(i, &c) };
    }
    // The copy of the process a failed `expect` walks its frames in
    // (decision 500) reaches nothing outside: the walk ends at the host.
    if rt::expect_copy() && i.name().starts_with("host.") {
        let stack = rt::newest_stack();
        rt::end_expect_copy(&stack);
    }
    if crate::host::handles(i) {
        // SAFETY: as the caller guarantees.
        return unsafe { crate::host::call(i, &c) };
    }
    // SAFETY: as the caller guarantees.
    system_status(|| unsafe { crate::intrinsics::call(i, &c) })
}

/// A `dict` or `set` (of type `t`) from a literal's `n` entries, moved in
/// from `items` (each the key's words, then the value's): a key given again
/// replaces the value and is itself dropped, as `insert` does.
///
/// # Safety
/// `items` must hold `n` entries of the type's key and value, and `out`
/// have room for a word.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_table(t: u32, items: *mut u8, n: u64, out: *mut u8) -> u8 {
    // SAFETY: as the caller guarantees.
    system_status(|| unsafe { table(t, items, n, out) })
}

/// # Safety
/// As `sisu_rt_table`.
unsafe fn table(t: u32, items: *mut u8, n: u64, out: *mut u8) -> u8 {
    let (k, v) = match ty(t).kind {
        Kind::Dict(k, v) => (k, v),
        Kind::Set(e) => (e, u32::MAX),
        _ => rt::internal("a table literal of an unknown type"),
    };
    let made = crate::dict::Table::new(k, v);
    // SAFETY: the table was just made.
    let table = unsafe { &mut *made };
    let (ksize, vsize) = (table.ksize() as usize, table.vsize() as usize);
    let mut status = abi::OK;
    // SAFETY: the entries are in `items`.
    unsafe {
        for i in 0..n as usize {
            let key = items.add(i * (ksize + vsize));
            let val = key.add(ksize);
            match table.position(key, k) {
                Some(j) => {
                    status = value::after_part(status, value::drop_value(table.val_at(j), v));
                    mem::copy(table.val_at(j), val, vsize);
                    status = value::after_part(status, value::drop_value(key, k));
                }
                None => {
                    let j = table.push_key(key);
                    mem::copy(table.val_at(j), val, vsize);
                }
            }
        }
        mem::set_word(out, 0, crate::dict::into_word(made));
    }
    status
}

/// The address of the value the dict at `d` holds for the key at `key`
/// (`d{k}`), written at `out`: a missing key panics (`SLOT_READ`), is
/// inserted, with a copy of the key and the value left to the caller
/// (`SLOT_WRITE`), or gives null (`SLOT_DROP`).
///
/// # Safety
/// `d` must hold a dict, `key` a key of type `key_ty`, and `out` be a word.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_dict_slot(d: *mut u8, key: *mut u8, key_ty: u32, mode: u32, out: *mut u8) -> u8 {
    // SAFETY: as the caller guarantees.
    system(|| unsafe { dict_slot(d, key, key_ty, mode, out) })
}

/// # Safety
/// As `sisu_rt_dict_slot`.
unsafe fn dict_slot(d: *mut u8, key: *mut u8, key_ty: u32, mode: u32, out: *mut u8) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe {
        let t = crate::dict::table(d);
        let at = match t.position(key, key_ty) {
            Some(i) => t.val_at(i),
            None if mode == abi::SLOT_WRITE => {
                let mut k = vec![0u64; (t.ksize() as usize).div_ceil(8)];
                let kp = k.as_mut_ptr() as *mut u8;
                crate::dict::copy_key(kp, key, key_ty);
                let i = t.push_key(kp);
                t.val_at(i)
            }
            None if mode == abi::SLOT_DROP => std::ptr::null_mut(),
            None => {
                let mut shown = Vec::new();
                let (p, kt) = value::target(key, key_ty);
                value::debug(&mut shown, p, kt);
                return rt::raise_msg(
                    msg::PANIC_MISSING_KEY,
                    &[("key", String::from_utf8_lossy(&shown).into_owned())],
                );
            }
        };
        mem::set_word(out, 0, at as u64);
    }
    abi::OK
}
