//! Thin wrappers over the memory-mapping system calls the stack strategies use.
//!
//! Every function that touches the address space is `unsafe` where the caller
//! must promise that the range belongs to it; the reservation itself is safe
//! because the kernel chooses a fresh range.

use std::io;
use std::ptr;
use std::sync::OnceLock;

/// The system page size (4096 on the x86-64 machines this harness targets).
pub fn page_size() -> usize {
    static PAGE: OnceLock<usize> = OnceLock::new();
    *PAGE.get_or_init(|| {
        // SAFETY: sysconf has no preconditions.
        let p = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if p > 0 && (p as usize).is_power_of_two() { p as usize } else { 4096 }
    })
}

/// Reserves `len` bytes (a multiple of the page size) of private anonymous memory,
/// readable and writable and committed lazily: no page is resident until touched,
/// and `MAP_NORESERVE` keeps the reservation out of the overcommit charge.
///
/// The range is marked `MADV_NOHUGEPAGE`: a stack backed by a 2 MiB transparent
/// huge page would make the resident size of a task 2 MiB. `MAP_STACK` does the
/// same on Linux 6.7 and later; the advice covers older kernels.
pub fn reserve(len: usize) -> io::Result<usize> {
    // SAFETY: an anonymous private mapping with a null address hint has no
    // preconditions; the kernel picks a range that overlaps nothing we own.
    let p = unsafe {
        libc::mmap(
            ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE | libc::MAP_STACK,
            -1,
            0,
        )
    };
    if p == libc::MAP_FAILED {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: [p, p+len) is the mapping just created; advice does not change its
    // contents. Failure (e.g. THP compiled out) is harmless and ignored.
    unsafe {
        libc::madvise(p, len, libc::MADV_NOHUGEPAGE);
    }
    Ok(p as usize)
}

/// Unmaps a range.
///
/// # Safety
/// `[addr, addr+len)` must be a range this process mapped and nothing may use it
/// afterwards (no live stack frames, no pending kernel I/O into it).
pub unsafe fn unmap(addr: usize, len: usize) -> io::Result<()> {
    // SAFETY: the caller owns the range and nothing references it any more.
    let r = unsafe { libc::munmap(addr as *mut libc::c_void, len) };
    if r == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Makes a range inaccessible (a guard). Splits the VMA, which is what makes
/// guard-page-per-task hit `vm.max_map_count`.
///
/// # Safety
/// The range must belong to the caller and hold no live data.
pub unsafe fn protect_none(addr: usize, len: usize) -> io::Result<()> {
    // SAFETY: the caller owns the range; making it inaccessible cannot violate
    // memory safety as long as nothing live is stored there.
    let r = unsafe { libc::mprotect(addr as *mut libc::c_void, len, libc::PROT_NONE) };
    if r == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Discards the contents of a range and returns its pages to the kernel
/// (`MADV_DONTNEED`); later reads see zero pages and writes fault in fresh ones.
/// Does not split VMAs.
///
/// # Safety
/// The range must belong to the caller and hold no live data (in particular no
/// live stack frames of a suspended task).
pub unsafe fn decommit(addr: usize, len: usize) -> io::Result<()> {
    // SAFETY: the caller guarantees the range is owned and dead.
    let r = unsafe { libc::madvise(addr as *mut libc::c_void, len, libc::MADV_DONTNEED) };
    if r == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}
