//! The views `*rest` patterns bind (decision 214): a `ref list[T]` to the
//! elements in place rather than to a list of copies. A view's header is
//! a heap block (a list's three words, a capacity of 0 marking it, and the
//! address of the buffer it views), freed when that buffer is: the
//! ownership checker keeps the binding from outliving the list it views.

use crate::mem::{self, List};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// The headers registered, so that a release looks at the table only when
/// there are any.
static REGISTERED: AtomicUsize = AtomicUsize::new(0);

/// The header of every empty view, kept for the run.
static EMPTY: OnceLock<usize> = OnceLock::new();

/// The headers of the views of each buffer, by its address: shared by the
/// threads, since a view a worker of `std.par` makes of the asking task's
/// list goes with that list (decision 333).
static VIEWS: Mutex<Option<HashMap<usize, Vec<usize>>>> = Mutex::new(None);

/// Runs `f` on the table of views.
fn with_views<R>(f: impl FnOnce(&mut HashMap<usize, Vec<usize>>) -> R) -> Option<R> {
    let mut g = VIEWS.lock().ok()?;
    Some(f(g.get_or_insert_with(HashMap::new)))
}

/// The words of a view's header: a list's three, then the buffer it views.
const HEADER: usize = 32;
const ROOT: u32 = 24;

/// A view of the elements `lo..hi` (of `size` bytes each) of the list
/// whose header is at `l`, which may itself be a view.
///
/// # Safety
/// `l` must hold a list's header with at least `hi` elements.
pub(crate) unsafe fn view(l: *mut u8, size: u32, lo: u64, hi: u64) -> *mut u8 {
    // SAFETY: as the caller guarantees.
    unsafe {
        let list = mem::at::<List>(l);
        if hi <= lo || list.ptr.is_null() {
            return *EMPTY.get_or_init(|| {
                let h = mem::alloc_kept(HEADER);
                (h as *mut List).write(List::empty());
                h as usize
            }) as *mut u8;
        }
        let root = if list.cap == 0 && list.len > 0 { mem::word(l, ROOT) } else { list.ptr as u64 };
        let h = mem::alloc(HEADER);
        (h as *mut List).write(List { ptr: list.at(lo, size), len: hi - lo, cap: 0 });
        mem::set_word(h, ROOT, root);
        let _ = with_views(|v| v.entry(root as usize).or_default().push(h as usize));
        REGISTERED.fetch_add(1, Relaxed);
        h
    }
}

/// Keeps `block`, a heap block of [`mem::alloc`], until the block at
/// `root` is released: a copy of a plain value that `view_plain` made of
/// bytes laid out otherwise in a value (decision 302).
pub(crate) fn hold(root: *mut u8, block: *mut u8) {
    let _ = with_views(|v| v.entry(root as usize).or_default().push(block as usize));
    REGISTERED.fetch_add(1, Relaxed);
}

/// The block at `p` is being released: the headers of the views of it go
/// with it.
pub(crate) fn released(p: *mut u8) {
    if REGISTERED.load(Relaxed) == 0 {
        return;
    }
    let headers = with_views(|v| v.remove(&(p as usize))).flatten();
    for h in headers.unwrap_or_default() {
        REGISTERED.fetch_sub(1, Relaxed);
        mem::release(h as *mut u8);
    }
}
