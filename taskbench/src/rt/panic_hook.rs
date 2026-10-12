//! Panic reporting for tasks on small stacks.
//!
//! The default panic hook formats the message and, with `RUST_BACKTRACE` set,
//! captures and symbolises a backtrace, which takes tens of KiB of stack. On a
//! strategy B or C stack there is no guard page and the probe reserve is a few
//! KiB, so the hook itself would overflow into the neighbouring slot (the harness
//! hit exactly this). Like Go's `systemstack`, this hook runs the reporting on
//! the worker thread's own stack through corosensei's `on_parent_stack`; only the
//! message and location are copied off the task stack first. Panics outside
//! stackful tasks go to the previous hook unchanged.

use std::sync::Once;

use super::api;

/// Installs the hook once per process.
pub(crate) fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let Some(task) = api::current_task().filter(|t| !t.stackless) else {
                prev(info);
                return;
            };
            // SAFETY: the panicking code is the running task, which owns `local`.
            let y = unsafe { task.local_mut() }.yielder;
            if y.is_null() {
                prev(info);
                return;
            }
            let payload = info.payload();
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string payload".to_string());
            let at = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_default();
            let id = task.id;
            let report = move || {
                eprintln!("task {id} panicked at {at}:\n{msg}\n(backtraces are not captured on task stacks)");
            };
            // SAFETY: `y` is this coroutine's yielder (set when it started); the
            // closure owns its data and runs on the parent (worker) stack.
            unsafe { (*y).on_parent_stack(report) };
        }));
    });
}
