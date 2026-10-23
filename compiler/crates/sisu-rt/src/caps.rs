//! Capabilities (06, "Capabilities"; decision 209), as the interpreter
//! keeps them (`sisu-interp/src/caps.rs`): a handle that may hold an
//! object of the program's is the address of a heap cell holding the
//! object as a `dyn` holds its value (the type's number, the value's
//! address, then the value), which every copy of the handle reaches; a
//! `Cap[T]` is the address of a cell holding the `T`; a `Lent` is its
//! handle. The run's test clock is a virtual clock of the scheduler, made
//! on first use.
//!
//! A cell counts the copies of its handle in the word before it (decision
//! 237): the generated code adds one for each copy it makes into a place,
//! an aggregate, an environment or an argument of a function (and of an
//! intrinsic that keeps it), a drop takes one, and the last drop drops the
//! object and frees the cell. A `Lent` counts nothing.

use crate::abi::{self, Intrinsic, Kind};
use crate::intrinsics::Call;
use crate::mem;
use crate::rt::{self, ty};
use crate::sched::with;

/// The host's objects behind the root capabilities, in the order of
/// [`abi::CAPABILITIES`]: a root handle's cell holds the address of one.
static HOST_OBJECTS: [u64; 7] = [0; 7];

/// The cells alive, by address: when each was made (a serial number), the
/// type of what it holds and where in it that is. A cell goes with its
/// handles' last copy; those a copy kept alive to the end of the run (one
/// an `own` closure captured, which is never dropped) are freed then,
/// running no Sisu code (decision 223).
static LIVE: std::sync::Mutex<std::collections::BTreeMap<usize, (u64, u32, usize)>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The serial number of the next cell.
static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The bytes of a cell's header, before its first word: the count.
const HEADER: usize = 8;

/// The run has ended: the cells left are being freed, and counts no longer
/// change.
static ENDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A new cell of `bytes`, holding a value of type `t` at `at`, with one
/// copy of its handle counted.
fn new_cell(bytes: usize, t: u32, at: usize) -> *mut u8 {
    let base = mem::alloc(HEADER + bytes.max(8));
    // SAFETY: the block has room for the header.
    unsafe { mem::set_word(base, 0, 1) };
    // SAFETY: the cell starts after the header.
    let cell = unsafe { base.add(HEADER) };
    let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut live) = LIVE.lock() {
        live.insert(cell as usize, (serial, t, at));
    }
    cell
}

/// One more copy of the handle on `cell` (decision 237).
///
/// # Safety
/// `cell` must be a live cell of this module.
pub(crate) unsafe fn retain_cell(cell: *mut u8) {
    if cell.is_null() || ENDED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    // SAFETY: the count is the word before the cell.
    unsafe {
        let n = mem::word(cell.sub(HEADER), 0);
        mem::set_word(cell.sub(HEADER), 0, n + 1);
    }
}

/// One copy fewer of the handle on `cell`: the last one drops what the
/// cell holds (running its glue, or, forgetting, none) and frees it.
///
/// # Safety
/// `cell` must be a live cell of this module.
pub(crate) unsafe fn release_cell(cell: *mut u8) -> u8 {
    if cell.is_null() || ENDED.load(std::sync::atomic::Ordering::Relaxed) {
        return abi::OK;
    }
    // SAFETY: the count is the word before the cell.
    let n = unsafe { mem::word(cell.sub(HEADER), 0) };
    if n > 1 {
        // SAFETY: as above.
        unsafe { mem::set_word(cell.sub(HEADER), 0, n - 1) };
        return abi::OK;
    }
    let found = LIVE.lock().ok().and_then(|mut live| live.remove(&(cell as usize)));
    let Some((_, t, at)) = found else { return abi::OK };
    // SAFETY: the cell holds a value of type `t` at `at`, which no copy of
    // its handle reaches any more; the value is dropped (or moved aside to
    // be) before the block is freed.
    unsafe {
        let status = crate::value::drop_value(cell.add(at), t);
        mem::release(cell.sub(HEADER));
        status
    }
}

/// The run is over: frees the cells still alive, last made first, running
/// no Sisu code, as the interpreter's host lets them go (decision 223).
pub(crate) fn release_kept() {
    ENDED.store(true, std::sync::atomic::Ordering::Relaxed);
    let mut cells: Vec<(usize, (u64, u32, usize))> =
        LIVE.lock().map(|mut k| std::mem::take(&mut *k).into_iter().collect()).unwrap_or_default();
    cells.sort_by_key(|(_, (serial, _, _))| std::cmp::Reverse(*serial));
    for (cell, (_, t, at)) in cells {
        // SAFETY: the cell holds a value of type `t` at `at`, which no code
        // reaches any more.
        unsafe {
            crate::value::forget_value((cell as *mut u8).add(at), t);
            mem::release((cell as *mut u8).sub(HEADER));
        }
    }
}

/// A new cell holding the `size` bytes at `src` (moved in) as a `dyn` of
/// type `t` holds its value: the handle.
///
/// # Safety
/// `src` must hold `size` bytes.
unsafe fn handle_cell(t: u64, src: *const u8, size: u32) -> u64 {
    let cell = new_cell(16 + size as usize, t as u32, 16);
    // SAFETY: the cell has room for two words and the value.
    unsafe {
        mem::set_word(cell, 0, t);
        mem::set_word(cell, 8, cell.add(16) as u64);
        mem::copy(cell.add(16), src, size as usize);
    }
    cell as u64
}

/// The number of the type the runtime knows by `name` only (the host's
/// objects), if the program has it.
pub(crate) fn opaque_type(name: &str) -> Option<u32> {
    let mut id = 0u32;
    loop {
        let t = rt::ty_checked(id)?;
        if matches!(&t.kind, Kind::Opaque(n) if n == name) {
            return Some(id);
        }
        id += 1;
    }
}

/// The `System` `main` receives: a handle on each of the host's objects.
/// A handle the program calls the host's operations through directly
/// (decision 194) never reads its cell.
pub(crate) fn root_system() -> Vec<u64> {
    abi::CAPABILITIES
        .iter()
        .zip(&HOST_OBJECTS)
        .map(|((_, object), cell)| {
            let t = opaque_type(object).map_or(u64::MAX, u64::from);
            let at = (cell as *const u64 as u64).to_ne_bytes();
            // SAFETY: the value is the object's address, one word.
            unsafe { handle_cell(t, at.as_ptr(), 8) }
        })
        .collect()
}

/// The intrinsics of capabilities, `Cap`, `Lent`, `System` and the test
/// clock; `None` for any other.
///
/// # Safety
/// The arguments are of the types the descriptor gives.
pub(crate) unsafe fn call(i: Intrinsic, c: &Call<'_>) -> Option<u8> {
    use Intrinsic as I;
    // SAFETY: each reads its arguments as their types say.
    unsafe {
        match i {
            // The handle takes the object over, with its type for dispatch.
            I::CapNew => {
                let t = c.targ(0);
                let h = handle_cell(u64::from(t), c.arg(0), ty(t).size);
                c.set_word(h);
            }
            I::CapCellNew => {
                let size = ty(c.ty(0)).size;
                let cell = new_cell(size as usize, c.ty(0), 0);
                mem::copy(cell, c.arg(0), size as usize);
                c.set_word(cell as u64);
            }
            // The value a `Cap`'s cell holds, by reference.
            I::CapGet => c.set_word(c.word(0)),
            // A lent capability is its handle (or its test clock).
            I::CapLend => {
                let (p, _) = c.place(0);
                mem::copy(c.ret, p, ty(c.ret_ty).size as usize);
            }
            I::SystemAssemble => {
                for k in 0..abi::CAPABILITIES.len() {
                    mem::set_word(c.ret, 8 * k as u32, c.word(k));
                }
            }
            I::ClockFake => c.set_word(u64::from(with(|s| s.fake_clock()))),
            I::TestClockAdvance | I::TestClockSet => {
                let clock = c.word(0) as u32;
                // A `Duration`'s nanoseconds, or a `DateTime`'s.
                let n = i128::from(c.int(1));
                with(|s| {
                    if let Some(k) = s.clocks.get_mut(clock as usize) {
                        k.now = if i == I::TestClockAdvance { k.now + n } else { n };
                    }
                    s.wake_due_timers();
                });
            }
            _ => return None,
        }
    }
    Some(abi::OK)
}
