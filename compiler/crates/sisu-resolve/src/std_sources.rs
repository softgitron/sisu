//! The standard-library modules of stage 0, embedded at build time. They
//! are Sisu source and go through the same front end as user code.

pub const PRELUDE: &str = include_str!("../../../std/prelude.sisu");
pub const CLI: &str = include_str!("../../../std/cli.sisu");
pub const MATH: &str = include_str!("../../../std/math.sisu");

/// The std module an import path names, as (module name, source).
pub fn module_for(path: &[&str]) -> Option<(&'static str, &'static str)> {
    match path {
        ["cli"] | ["std", "x", "cli"] => Some(("cli", CLI)),
        ["std", "math"] | ["math"] => Some(("math", MATH)),
        _ => None,
    }
}
