//! The `sim` profile's faults under a native test (17, "Faults come from
//! types"; decision 502), as the interpreter's `sim_fault` injects them
//! (`sisu-interp/src/exec.rs`): the driver puts `test.sim_fault[R](handle,
//! path)` before each call through `FsOps` or `NetOps` on a handle, and on
//! its `true` gives the call's destination `test.sim_fault_error()` in
//! place of the call. At the run's rate (`SISU_SIM_FAULTS`, per million), a
//! call on one of the simulation's own objects (`cap.SimFs`,
//! `cap.SimNetwork`, `sim.SimNet`) fails with an error of its declared set
//! but `TaskError`, never `Revoked`, drawn from a generator of its own
//! seeded as the interpreter's, so a seed gives the same faults.

use crate::abi::Kind;
use crate::intrinsics::Call;
use crate::mem;
use crate::rt::{self, ty};
use crate::sched::Rng;
use std::sync::Mutex;

/// The generator of the faults, made on first use, and the fault decided
/// and not yet written: the error set's type, the variant, and the path.
struct Faults {
    rng: Rng,
    rate: u64,
    pending: Option<(u32, usize, Option<Vec<u8>>)>,
}

static FAULTS: Mutex<Option<Faults>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Faults) -> R) -> R {
    let mut g = FAULTS.lock().unwrap_or_else(|p| p.into_inner());
    let faults = g.get_or_insert_with(|| {
        let env = |k: &str| std::env::var(k).ok().and_then(|v| v.trim().parse::<u64>().ok());
        let seed = env("SISU_SCHEDULE_SEED").unwrap_or(0);
        Faults { rng: Rng::new(seed ^ 0x5157_FA17), rate: env("SISU_SIM_FAULTS").unwrap_or(0), pending: None }
    });
    f(faults)
}

/// The name of the type of the object behind the handle at `p`, a word
/// holding the address of its cell (`caps.rs`).
///
/// # Safety
/// `p` must hold a capability handle.
unsafe fn object_name(p: *mut u8) -> Option<&'static str> {
    // SAFETY: a handle is the address of its cell, which starts with the
    // number of the object's type (a root handle's, `u64::MAX` when the
    // program has none).
    let t = unsafe {
        let cell = mem::word(p, 0) as *mut u8;
        if cell.is_null() {
            return None;
        }
        mem::word(cell, 0)
    };
    match &rt::ty_checked(u32::try_from(t).ok()?)?.kind {
        Kind::Struct { name, .. } | Kind::Enum { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// The name of the error set (or enum) of type `t`.
fn type_name(t: u32) -> Option<&'static str> {
    match &rt::ty_checked(t)?.kind {
        Kind::Enum { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// `test.sim_fault[R](handle, path?)`: whether the call that follows fails
/// with a fault; `R` is its result's type.
///
/// # Safety
/// The arguments are a handle (or a reference to one) and, when given, a
/// path.
pub(crate) unsafe fn decide(c: &Call<'_>) -> bool {
    // SAFETY: as the caller guarantees.
    let (handle, _) = unsafe { c.place(0) };
    // SAFETY: as above.
    let object = unsafe { object_name(handle) };
    if !matches!(object, Some("cap.SimFs" | "cap.SimNetwork" | "sim.SimNet")) {
        return false;
    }
    let errs: Vec<u32> = match &ty(c.targ(0)).kind {
        Kind::Result(_, e) => match &ty(*e).kind {
            Kind::ErrSet(members) => members.clone(),
            Kind::Enum { error: true, .. } => vec![*e],
            _ => return false,
        },
        _ => return false,
    };
    let errs: Vec<u32> = errs.into_iter().filter(|e| type_name(*e) != Some("TaskError")).collect();
    let path = if c.args.len() > 1 {
        // SAFETY: the second argument is a path.
        unsafe { crate::value::text(c.arg(1), c.ty(1)) }.map(<[u8]>::to_vec)
    } else {
        None
    };
    with(|f| {
        if f.rate == 0 || errs.is_empty() || f.rng.next() % 1_000_000 >= f.rate {
            return false;
        }
        let decl = errs[f.rng.below(errs.len())];
        let Kind::Enum { variants, .. } = &ty(decl).kind else { return false };
        // The simulation's objects are never revocable (decision 426).
        let possible: Vec<usize> = (0..variants.len()).filter(|&v| variants[v].name != "Revoked").collect();
        if possible.is_empty() {
            return false;
        }
        let k = possible[f.rng.below(possible.len())];
        f.pending = Some((decl, k, path));
        true
    })
}

/// `test.sim_fault_error()`: `Err` of the fault [`decide`] chose, into the
/// call's result: a path field is the call's path (or `(simulated)`), a
/// text says that the simulation made it, a number is 0.
///
/// # Safety
/// The result has room for a value of its type, a `Result`.
pub(crate) unsafe fn write(c: &Call<'_>) {
    let Some((decl, k, path)) = with(|f| f.pending.take()) else {
        rt::internal("a fault written that was not decided")
    };
    let Kind::Enum { variants, .. } = &ty(decl).kind else { rt::internal("a fault of no error set") };
    // SAFETY: a result is its tag, then the error: its type's number, its
    // trace and its tag, then the variant's fields.
    unsafe {
        mem::set_word(c.ret, 0, 1);
        let e = c.ret.add(8);
        mem::set_word(e, crate::abi::ERROR_DECL, u64::from(decl));
        mem::set_word(e, crate::abi::ERROR_TRACE, 0);
        mem::set_word(e, crate::abi::ERROR_TAG, k as u64);
        for f in &variants[k].fields {
            let at = e.add(f.offset as usize);
            match &ty(f.ty).kind {
                Kind::Path => crate::value::write_string(at, path.as_deref().unwrap_or(b"(simulated)")),
                Kind::String => crate::value::write_string(at, b"a fault the simulation made"),
                _ => {
                    for w in 0..ty(f.ty).size / 8 {
                        mem::set_word(at, 8 * w, 0);
                    }
                }
            }
        }
    }
}
