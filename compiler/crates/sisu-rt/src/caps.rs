//! Capabilities (06, "Capabilities"; decision 209), as the interpreter
//! keeps them (`sisu-interp/src/caps.rs`): a handle that may hold an
//! object of the program's is the address of a heap cell holding the
//! object as a `dyn` holds its value (the type's number, the value's
//! address, then the value), which every copy of the handle reaches and
//! nothing frees; a `Cap[T]` is the address of a cell holding the `T`;
//! a `Lent` is its handle. The run's test clock is a virtual clock of the
//! scheduler, made on first use.

use crate::abi::{self, Intrinsic, Kind};
use crate::intrinsics::Call;
use crate::mem;
use crate::rt::{self, ty};
use crate::sched::with;

/// The host's objects behind the root capabilities, in the order of
/// [`abi::CAPABILITIES`]: a root handle's cell holds the address of one.
static HOST_OBJECTS: [u64; 7] = [0; 7];

/// A new cell holding the `size` bytes at `src` (moved in) as a `dyn` of
/// type `t` holds its value: the handle.
///
/// # Safety
/// `src` must hold `size` bytes.
unsafe fn handle_cell(t: u64, src: *const u8, size: u32) -> u64 {
    let cell = mem::alloc_kept(16 + size as usize);
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
                let cell = mem::alloc_kept((size as usize).max(8));
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
