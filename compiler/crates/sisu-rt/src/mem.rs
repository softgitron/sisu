//! Raw memory: the heap, and the headers of texts and lists as
//! [`crate::abi`] lays them out; the count of the program's allocations
//! that are live, which `SISU_RT_ALLOCS` reports at exit.

use std::sync::atomic::{AtomicI64, Ordering::Relaxed};

/// The blocks of this module and the runtime objects of values
/// (iterators, tables, arenas, the nodes of return traces) that are live.
/// The cells kept for the whole run ([`alloc_kept`]) are not counted.
static LIVE: AtomicI64 = AtomicI64::new(0);

/// The most of them live at once so far (decision 229).
static PEAK: AtomicI64 = AtomicI64::new(0);

/// The number of the program's allocations live now.
pub(crate) fn live() -> i64 {
    LIVE.load(Relaxed)
}

/// The most of the program's allocations that were live at once.
pub(crate) fn peak() -> i64 {
    PEAK.load(Relaxed)
}

/// One more allocation is live.
fn counted() {
    let now = LIVE.fetch_add(1, Relaxed) + 1;
    PEAK.fetch_max(now, Relaxed);
}

/// A runtime object of a value counts itself live while it exists.
pub(crate) struct Live(());

impl Live {
    pub(crate) fn new() -> Live {
        counted();
        Live(())
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        LIVE.fetch_sub(1, Relaxed);
    }
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn realloc(p: *mut u8, size: usize) -> *mut u8;
    fn free(p: *mut u8);
    fn memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn posix_memalign(out: *mut *mut u8, align: usize, size: usize) -> i32;
}

/// A block of `bytes` aligned to `align`, a power of two, counted as the
/// program's: what `Global.acquire` gives out (decision 302), released with
/// [`release`]. Null when there is no memory.
pub(crate) fn alloc_aligned(bytes: usize, align: usize) -> *mut u8 {
    let mut p: *mut u8 = std::ptr::null_mut();
    // SAFETY: the alignment is a power of two and a multiple of a pointer's
    // size, as posix_memalign asks; a zero size still gives a block.
    let rc = unsafe { posix_memalign(&mut p, align.max(16), bytes.max(1)) };
    if rc != 0 || p.is_null() {
        return std::ptr::null_mut();
    }
    counted();
    p
}

/// `bytes` of heap memory, aligned for a word; null for none.
pub(crate) fn alloc(bytes: usize) -> *mut u8 {
    let p = alloc_kept(bytes);
    if !p.is_null() {
        counted();
    }
    p
}

/// A heap block that lives as long as the run and is not counted: the
/// empty view (decision 214).
pub(crate) fn alloc_kept(bytes: usize) -> *mut u8 {
    if bytes == 0 {
        return std::ptr::null_mut();
    }
    // SAFETY: malloc may be called with any size; it returns memory
    // aligned for any scalar or null.
    let p = unsafe { malloc(bytes) };
    if p.is_null() {
        crate::rt::abort("out of memory");
    }
    p
}

/// Grows (or makes) a heap block.
pub(crate) fn grow(p: *mut u8, bytes: usize) -> *mut u8 {
    if bytes == 0 {
        return p;
    }
    // SAFETY: `p` is null or a block this module allocated.
    let q = unsafe { realloc(p, bytes) };
    if q.is_null() {
        crate::rt::abort("out of memory");
    }
    if p.is_null() {
        counted();
    }
    q
}

pub(crate) fn release(p: *mut u8) {
    if !p.is_null() {
        LIVE.fetch_sub(1, Relaxed);
        crate::views::released(p);
        // SAFETY: `p` is a block this module allocated, released once.
        unsafe { free(p) }
    }
}

/// Copies `n` bytes; the regions may overlap.
///
/// # Safety
/// Both regions must be valid for `n` bytes.
pub(crate) unsafe fn copy(dst: *mut u8, src: *const u8, n: usize) {
    if n > 0 && !std::ptr::eq(dst, src) {
        // SAFETY: the caller guarantees both regions.
        unsafe {
            memmove(dst, src, n);
        }
    }
}

/// Reads the word at `p + off`.
///
/// # Safety
/// `p + off` must be a valid, aligned word.
pub(crate) unsafe fn word(p: *const u8, off: u32) -> u64 {
    // SAFETY: the caller guarantees the word.
    unsafe { (p.add(off as usize) as *const u64).read() }
}

/// Writes the word at `p + off`.
///
/// # Safety
/// `p + off` must be a valid, aligned word.
pub(crate) unsafe fn set_word(p: *mut u8, off: u32, v: u64) {
    // SAFETY: the caller guarantees the word.
    unsafe { (p.add(off as usize) as *mut u64).write(v) }
}

/// A `str`: a pointer to UTF-8 bytes and their length.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Str {
    pub ptr: *const u8,
    pub len: u64,
}

/// A `String` or a `Path`: a heap buffer, its length and capacity.
#[repr(C)]
pub(crate) struct Text {
    pub ptr: *mut u8,
    pub len: u64,
    pub cap: u64,
}

/// A `list`: a heap buffer of elements, their number and the capacity.
#[repr(C)]
pub(crate) struct List {
    pub ptr: *mut u8,
    pub len: u64,
    pub cap: u64,
}

impl Str {
    /// The bytes it views.
    ///
    /// # Safety
    /// The viewed bytes must outlive the slice.
    pub(crate) unsafe fn bytes<'a>(self) -> &'a [u8] {
        if self.len == 0 || self.ptr.is_null() {
            return &[];
        }
        // SAFETY: the caller guarantees the bytes.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len as usize) }
    }
}

impl Text {
    pub(crate) fn empty() -> Text {
        Text { ptr: std::ptr::null_mut(), len: 0, cap: 0 }
    }

    /// A new text holding these bytes.
    pub(crate) fn from_bytes(b: &[u8]) -> Text {
        let mut t = Text::empty();
        t.push(b);
        t
    }

    pub(crate) fn push(&mut self, b: &[u8]) {
        if b.is_empty() {
            return;
        }
        let need = self.len + b.len() as u64;
        if need > self.cap {
            let cap = need.max(self.cap * 2).max(16);
            self.ptr = grow(self.ptr, cap as usize);
            self.cap = cap;
        }
        // SAFETY: the buffer holds `need` bytes, and `b` is not in it
        // (a view of this text cannot be pushed onto it).
        unsafe {
            copy(self.ptr.add(self.len as usize), b.as_ptr(), b.len());
        }
        self.len = need;
    }

    /// The bytes it holds.
    ///
    /// # Safety
    /// The text must outlive the slice and not change meanwhile.
    pub(crate) unsafe fn bytes<'a>(&self) -> &'a [u8] {
        if self.len == 0 || self.ptr.is_null() {
            return &[];
        }
        // SAFETY: the caller guarantees the buffer.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len as usize) }
    }
}

impl List {
    pub(crate) fn empty() -> List {
        List { ptr: std::ptr::null_mut(), len: 0, cap: 0 }
    }

    /// The address of element `i` of elements of `size` bytes.
    pub(crate) fn at(&self, i: u64, size: u32) -> *mut u8 {
        // SAFETY: the caller indexes within the buffer (or one past it).
        unsafe { self.ptr.add((i * u64::from(size)) as usize) }
    }

    /// Room for `extra` more elements.
    pub(crate) fn reserve(&mut self, extra: u64, size: u32) {
        let need = self.len + extra;
        if need > self.cap {
            let cap = need.max(self.cap * 2).max(4);
            self.ptr = grow(self.ptr, (cap * u64::from(size.max(1))) as usize);
            self.cap = cap;
        }
    }

    /// Appends the element at `src`, moving its bytes in.
    ///
    /// # Safety
    /// `src` must hold `size` bytes outside the list's buffer.
    pub(crate) unsafe fn push(&mut self, src: *const u8, size: u32) {
        self.reserve(1, size);
        // SAFETY: there is room for one more element.
        unsafe { copy(self.at(self.len, size), src, size as usize) };
        self.len += 1;
    }
}

/// The value at `p` as a reference to `T`.
///
/// # Safety
/// `p` must point to a valid, aligned `T`.
pub(crate) unsafe fn at<'a, T>(p: *mut u8) -> &'a mut T {
    // SAFETY: the caller guarantees the value.
    unsafe { &mut *(p as *mut T) }
}

/// The `str` at `p`.
///
/// # Safety
/// `p` must hold a `str`.
pub(crate) unsafe fn str_at(p: *const u8) -> Str {
    // SAFETY: the caller guarantees the value.
    unsafe { (p as *const Str).read() }
}
