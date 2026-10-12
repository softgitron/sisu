//! Optional hardware counters (`perf_event_open`) for `probe-overhead`.
//!
//! Instructions retired are deterministic for a fixed binary and input, so on a
//! machine with a PMU the probe overhead can be stated without the wall-clock
//! noise of a shared VM. Where the kernel exposes no hardware events (many VMs,
//! including the development machine of this harness) `open` fails and the
//! benchmark says so in the CSV.

use std::io;

/// The prefix of `struct perf_event_attr` this module needs (128 bytes is
/// PERF_ATTR_SIZE_VER7's size; the kernel accepts any known size).
#[repr(C)]
struct PerfEventAttr {
    type_: u32,
    size: u32,
    config: u64,
    sample_period: u64,
    sample_type: u64,
    read_format: u64,
    flags: u64,
    rest: [u64; 10],
}

const PERF_TYPE_HARDWARE: u32 = 0;
const PERF_COUNT_HW_CPU_CYCLES: u64 = 0;
const PERF_COUNT_HW_INSTRUCTIONS: u64 = 1;
const FLAG_DISABLED: u64 = 1;
const FLAG_EXCLUDE_KERNEL: u64 = 1 << 5;
const FLAG_EXCLUDE_HV: u64 = 1 << 6;
const IOC_ENABLE: libc::c_ulong = 0x2400;
const IOC_DISABLE: libc::c_ulong = 0x2401;
const IOC_RESET: libc::c_ulong = 0x2403;

fn open_counter(config: u64) -> io::Result<i32> {
    let attr = PerfEventAttr {
        type_: PERF_TYPE_HARDWARE,
        size: std::mem::size_of::<PerfEventAttr>() as u32,
        config,
        sample_period: 0,
        sample_type: 0,
        read_format: 0,
        flags: FLAG_DISABLED | FLAG_EXCLUDE_KERNEL | FLAG_EXCLUDE_HV,
        rest: [0; 10],
    };
    // SAFETY: perf_event_open reads a valid attribute struct; pid 0 / cpu -1
    // counts the calling thread on any CPU; no group, no flags.
    let fd = unsafe { libc::syscall(libc::SYS_perf_event_open, &attr as *const PerfEventAttr, 0, -1, -1, 0) };
    if fd < 0 { Err(io::Error::last_os_error()) } else { Ok(fd as i32) }
}

/// User-space cycles and instructions of the calling thread.
pub struct Counters {
    cycles: i32,
    instructions: i32,
}

impl Counters {
    /// Opens both counters, or explains why not.
    pub fn open() -> Result<Counters, String> {
        let cycles =
            open_counter(PERF_COUNT_HW_CPU_CYCLES).map_err(|e| format!("hardware counters unavailable: {e}"))?;
        match open_counter(PERF_COUNT_HW_INSTRUCTIONS) {
            Ok(instructions) => Ok(Counters { cycles, instructions }),
            Err(e) => {
                // SAFETY: closing the descriptor opened above.
                unsafe { libc::close(cycles) };
                Err(format!("hardware counters unavailable: {e}"))
            }
        }
    }

    /// Runs `f` with the counters enabled; returns its result, cycles and
    /// instructions. The calling thread must not change during `f` (a task that
    /// does not yield).
    pub fn measure<R>(&self, f: impl FnOnce() -> R) -> (R, u64, u64) {
        for fd in [self.cycles, self.instructions] {
            // SAFETY: valid perf descriptors; these ioctls take no argument.
            unsafe {
                libc::ioctl(fd, IOC_RESET, 0);
                libc::ioctl(fd, IOC_ENABLE, 0);
            }
        }
        let r = f();
        let mut out = [0u64; 2];
        for (k, fd) in [self.cycles, self.instructions].into_iter().enumerate() {
            // SAFETY: valid descriptor; reads one u64 counter value.
            unsafe {
                libc::ioctl(fd, IOC_DISABLE, 0);
                libc::read(fd, &mut out[k] as *mut u64 as *mut libc::c_void, 8);
            }
        }
        (r, out[0], out[1])
    }
}

impl Drop for Counters {
    fn drop(&mut self) {
        // SAFETY: closing descriptors this object owns.
        unsafe {
            libc::close(self.cycles);
            libc::close(self.instructions);
        }
    }
}
