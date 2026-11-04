//! `sisu test --native` (decision 460): a program's tests compiled by the
//! native backend into one executable and run as the interpreter runs
//! them. The driver adds a dispatcher, `tests#main`, to the SIR stage 1's
//! `sir --tests` printed: it reads which test to run from the runtime
//! (`test.entry`, `SISU_TEST_ENTRY`), makes what the test takes as the
//! interpreter's `run_test` makes it (the real `System`, a simulated one,
//! or fresh fakes of the capabilities it names, decision 241) and calls
//! it. Each test is a process of its own, those that take no real
//! `System` in parallel; how it ended is written to the file
//! `SISU_TEST_REPORT` names (a panic's report, or the hole that blocked
//! it), apart from what it wrote, which is held and printed in
//! declaration order as the interpreter's parallel run prints it.

use crate::native::{self, Profile};
use sisu_diag::msg;
use sisu_interp::{Host, Options, RunReport, TestResult};
use sisu_sir as sir;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The dispatcher's name, which the runtime leaves out of stack traces.
const DISPATCHER: &str = "tests#main";

/// The fields of a `System`, in order, as the interpreter's `test_fakes`
/// takes them apart.
const SYSTEM_FIELDS: [&str; 7] = ["Fs", "Net", "Clock", "Rand", "Env", "Proc", "Stdin"];

/// Whether `sisu test` runs its tests natively, and in which profile.
static NATIVE: Mutex<Option<Profile>> = Mutex::new(None);

/// The executables built in this process, by what their dispatcher holds
/// (the case seed and the profile of `sim`), and the directory they are in.
#[allow(clippy::type_complexity)]
static BUILT: Mutex<(Option<PathBuf>, Vec<((u64, bool), PathBuf)>)> = Mutex::new((None, Vec::new()));

pub(crate) fn set(profile: Option<Profile>) {
    *NATIVE.lock().unwrap_or_else(|p| p.into_inner()) = profile;
}

pub(crate) fn chosen() -> Option<Profile> {
    *NATIVE.lock().unwrap_or_else(|p| p.into_inner())
}

fn text(id: msg::MsgId, params: &[(&str, String)]) -> String {
    let mut s = sisu_diag::Catalog::builtin().render(id.id(), params);
    s.push('\n');
    s
}

/// Why the options ask for what a native run does not do, if they do:
/// nothing since PCT, the exhaustive search and the faults run natively
/// (decisions 501 and 502).
pub(crate) fn refused(_options: &Options) -> Option<&'static str> {
    None
}

/// The removal of the executables this process built.
pub(crate) fn clean() {
    let mut b = BUILT.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(dir) = b.0.take() {
        let _ = std::fs::remove_dir_all(dir);
    }
    b.1.clear();
}

/// The program with the dispatcher of its tests as its entry.
fn with_dispatcher(prog: &sir::Program, case_seed: u64, sim: bool) -> Result<sir::Program, String> {
    let function = |name: &str| prog.functions.iter().find(|f| f.name == name);
    let real_system = |f: &sir::Function| !sim && sir::takes_real_system(f);
    let entry_system = prog.tests.iter().filter_map(|(_, f)| function(f)).any(real_system);
    let mut locals: Vec<String> = vec!["()".into(), if entry_system { "System" } else { "()" }.into()];
    let mut local = |ty: &str| {
        locals.push(ty.to_string());
        locals.len() - 1
    };
    let selected = local("int");
    let mut blocks = String::new();
    let mut arms = Vec::new();
    for (k, (_, name)) in prog.tests.iter().enumerate() {
        let Some(f) = function(name) else { return Err(format!("the test function `{name}`")) };
        let bb = k + 1;
        arms.push(format!("{k}: bb{bb}"));
        let mut body = String::new();
        let mut args = Vec::new();
        let params: Vec<String> = f
            .locals
            .iter()
            .skip(1)
            .take(f.param_count as usize)
            .map(|l| match &l.ty {
                sir::Type::Named(n, _) => n.clone(),
                _ => String::new(),
            })
            .collect();
        if entry_system && !real_system(f) {
            body.push_str("    drop _1\n");
        }
        if real_system(f) {
            args.push("move _1".to_string());
        } else if sir::takes_real_system(f) {
            // In the `sim` profile a test's `System` is simulated (17).
            if function("System.sim").is_none() {
                return Err("a simulated `System` the program does not keep".into());
            }
            let s = local("System");
            body.push_str(&format!("    _{s} = call fn System.sim(const {case_seed})\n"));
            args.push(format!("move _{s}"));
        } else if !params.is_empty() {
            let maker = if sim { "System.sim" } else { "System.fake" };
            if function(maker).is_none() {
                return Err(format!("`{maker}`, which the program does not keep"));
            }
            let s = local("System");
            body.push_str(&format!("    _{s} = call fn {maker}(const {case_seed})\n"));
            let mut taken = [false; 7];
            for p in &params {
                let a = local(p);
                match (p.as_str(), SYSTEM_FIELDS.iter().position(|h| h == p)) {
                    ("Net", _) if !sim => {
                        if function("Net.fake").is_none() {
                            return Err("`Net.fake`, which the program does not keep".into());
                        }
                        body.push_str(&format!("    _{a} = call fn Net.fake()\n"));
                    }
                    ("TestClock", _) => body.push_str(&format!("    _{a} = call intrinsic Clock.fake()\n")),
                    (_, Some(field)) if !taken[field] => {
                        taken[field] = true;
                        body.push_str(&format!("    _{a} = move _{s}.{field}\n"));
                    }
                    _ => return Err(format!("a test that takes a `{p}`")),
                }
                args.push(format!("move _{a}"));
            }
            for (field, t) in taken.iter().enumerate() {
                if !t {
                    body.push_str(&format!("    drop _{s}.{field}\n"));
                }
            }
        }
        body.push_str(&format!("    _0 = call fn {name}({})\n    return\n", args.join(", ")));
        blocks.push_str(&format!("  bb{bb}:\n{body}"));
    }
    let last = prog.tests.len() + 1;
    let mut text = format!("sir 0\n\nintrinsic test.entry\nintrinsic Clock.fake\n\nfn {DISPATCHER}(");
    text.push_str(if entry_system { "1" } else { "0" });
    text.push_str(") source 0 {\n");
    for (i, t) in locals.iter().enumerate() {
        text.push_str(&format!("    let _{i}: {t}\n"));
    }
    text.push_str(&format!(
        "  bb0:\n    _{selected} = call intrinsic test.entry()\n    switch copy _{selected} [{}{}else: bb{last}]\n",
        arms.join(", "),
        if arms.is_empty() { "" } else { ", " }
    ));
    text.push_str(&blocks);
    text.push_str(&format!("  bb{last}:\n    unreachable\n}}\n"));
    if sim {
        text.push_str("\nintrinsic test.sim_fault\nintrinsic test.sim_fault_error\n");
    }
    let extra = sir::read::read(&text).map_err(|e| format!("its dispatcher ({e:?})"))?;
    let mut out = prog.clone();
    if sim {
        for f in &mut out.functions {
            with_faults(f);
        }
    }
    for d in extra.intrinsics {
        if !out.intrinsics.iter().any(|i| i.name == d.name) {
            out.intrinsics.push(d);
        }
    }
    out.functions.extend(extra.functions);
    out.entry =
        Some(sir::Entry { function: DISPATCHER.into(), takes_system: entry_system, returns_result: false });
    Ok(out)
}

/// The `sim` profile's faults (17; decision 502), where the interpreter
/// injects them at a call (`exec.rs`, `sim_fault`): each call through
/// `FsOps` or `NetOps` on an `Fs` or a `Net` asks the runtime first
/// (`test.sim_fault[R](handle, path)`, `R` the call's result), and on its
/// `true` its destination is the fault (`test.sim_fault_error`), the
/// arguments it moved dropped, in place of the call. The new blocks follow
/// the call's own, the later ones renumbered after them, so that a jump
/// that was forward stays forward and the yield points (the back edges)
/// are the interpreter's.
fn with_faults(f: &mut sir::Function) {
    use sir::{
        BasicBlock, BlockId, Callee, Operand, Place, Rvalue, Statement, StatementKind, Terminator, Type,
    };
    let faulted = |st: &Statement, locals: &[sir::LocalDecl]| -> bool {
        let StatementKind::Call { dest, callee: Callee::Trait(name, tys), .. } = &st.kind else {
            return false;
        };
        let ops = name.starts_with("FsOps.") || name.starts_with("NetOps.");
        let handle =
            matches!(tys.first(), Some(Type::Named(n, a)) if a.is_empty() && (n == "Fs" || n == "Net"));
        let result = dest.proj.is_empty()
            && matches!(locals.get(dest.local.0 as usize).map(|l| &l.ty), Some(Type::Named(n, _)) if n == "Result");
        ops && handle && result
    };
    if !f.blocks.iter().any(|b| b.stmts.iter().any(|st| faulted(st, &f.locals))) {
        return;
    }
    // Each old block becomes one or more new ones, in order.
    let mut pieces: Vec<Vec<BasicBlock>> = Vec::new();
    let mut locals = f.locals.clone();
    // A reference to an argument's place: a reference is copied, another
    // value borrowed into a new local.
    fn by_ref(
        op: &Operand,
        cur: &mut BasicBlock,
        locals: &mut Vec<sir::LocalDecl>,
        loc: Option<sir::Loc>,
    ) -> Option<Operand> {
        let place = match op {
            Operand::Copy(p) | Operand::Move(p) if p.proj.is_empty() => p,
            _ => return None,
        };
        let ty = locals.get(place.local.0 as usize)?.ty.clone();
        if matches!(ty, Type::Ref(..)) {
            return Some(Operand::Copy(place.clone()));
        }
        locals.push(sir::LocalDecl::new(Type::Ref(Box::new(ty), false), None));
        let r = Place::local(sir::LocalId(locals.len() as u32 - 1));
        cur.stmts.push(Statement {
            kind: StatementKind::Assign(r.clone(), Rvalue::Ref(place.clone(), false)),
            loc,
        });
        Some(Operand::Copy(r))
    }
    for mut b in std::mem::take(&mut f.blocks) {
        let stmts = std::mem::take(&mut b.stmts);
        let mut out: Vec<BasicBlock> = Vec::new();
        let mut cur = BasicBlock { stmts: Vec::new(), term: Terminator::Unreachable, ..b.clone() };
        for st in stmts {
            if !faulted(&st, &locals) {
                cur.stmts.push(st);
                continue;
            }
            let StatementKind::Call { dest, args, .. } = &st.kind else { continue };
            let loc = st.loc;
            let is_path = |op: &Operand, locals: &[sir::LocalDecl]| match op {
                Operand::Copy(p) | Operand::Move(p) if p.proj.is_empty() => {
                    let mut t = locals.get(p.local.0 as usize).map(|l| &l.ty);
                    while let Some(Type::Ref(inner, _)) = t {
                        t = Some(inner);
                    }
                    matches!(t, Some(Type::Named(n, a)) if n == "Path" && a.is_empty())
                }
                _ => false,
            };
            let mut asked = Vec::new();
            let Some(handle) = args.first().and_then(|a| by_ref(a, &mut cur, &mut locals, loc)) else {
                cur.stmts.push(st);
                continue;
            };
            asked.push(handle);
            let path = args.iter().skip(1).find(|a| is_path(a, &locals)).cloned();
            if let Some(path) = path.and_then(|a| by_ref(&a, &mut cur, &mut locals, loc)) {
                asked.push(path);
            }
            let result = locals[dest.local.0 as usize].ty.clone();
            locals.push(sir::LocalDecl::new(Type::Bool, None));
            let flag = Place::local(sir::LocalId(locals.len() as u32 - 1));
            cur.stmts.push(Statement {
                kind: StatementKind::Call {
                    dest: flag.clone(),
                    callee: Callee::Intrinsic("test.sim_fault".into(), vec![result.clone()]),
                    args: asked,
                },
                loc,
            });
            // The ids are fixed up below: these say which piece is meant.
            let base = out.len() as u32;
            cur.term = Terminator::Switch {
                value: Operand::Copy(flag),
                cases: vec![(1, BlockId(base + 1))],
                default: BlockId(base + 2),
            };
            let mut fault = BasicBlock {
                stmts: vec![Statement {
                    kind: StatementKind::Call {
                        dest: dest.clone(),
                        callee: Callee::Intrinsic("test.sim_fault_error".into(), vec![result]),
                        args: Vec::new(),
                    },
                    loc,
                }],
                term: Terminator::Goto(BlockId(base + 3)),
                ..b.clone()
            };
            for a in args {
                if let Operand::Move(p) = a {
                    fault
                        .stmts
                        .push(Statement { kind: StatementKind::Drop { place: p.clone(), flag: None }, loc });
                }
            }
            let call = BasicBlock {
                stmts: vec![st.clone()],
                term: Terminator::Goto(BlockId(base + 3)),
                ..b.clone()
            };
            out.push(std::mem::replace(
                &mut cur,
                BasicBlock { stmts: Vec::new(), term: Terminator::Unreachable, ..b.clone() },
            ));
            out.push(fault);
            out.push(call);
        }
        cur.term = b.term;
        out.push(cur);
        pieces.push(out);
    }
    // The new number of each old block, and of each piece.
    let mut first = Vec::with_capacity(pieces.len());
    let mut n = 0u32;
    for p in &pieces {
        first.push(n);
        n += p.len() as u32;
    }
    let old = |id: BlockId| BlockId(first.get(id.0 as usize).copied().unwrap_or(id.0));
    for (k, p) in pieces.into_iter().enumerate() {
        let last = p.len() - 1;
        for (j, mut b) in p.into_iter().enumerate() {
            b.unwind = b.unwind.map(old);
            if j == last {
                match &mut b.term {
                    Terminator::Goto(t) => *t = old(*t),
                    Terminator::Switch { cases, default, .. } => {
                        for (_, t) in cases.iter_mut() {
                            *t = old(*t);
                        }
                        *default = old(*default);
                    }
                    _ => {}
                }
            } else {
                let local = |t: BlockId| BlockId(first[k] + t.0);
                match &mut b.term {
                    Terminator::Goto(t) => *t = local(*t),
                    Terminator::Switch { cases, default, .. } => {
                        for (_, t) in cases.iter_mut() {
                            *t = local(*t);
                        }
                        *default = local(*default);
                    }
                    _ => {}
                }
            }
            f.blocks.push(b);
        }
    }
    f.locals = locals;
}

/// The executable that runs the program's tests, built once for each
/// case seed and profile of `sim` in this process.
fn executable(
    prog: &sir::Program,
    options: &Options,
    profile: Profile,
    host: &mut dyn Host,
) -> Result<PathBuf, i32> {
    let key = (options.case_seed, options.sim.is_some());
    let mut b = BUILT.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((_, exe)) = b.1.iter().find(|(k, _)| *k == key) {
        return Ok(exe.clone());
    }
    let dir =
        b.0.get_or_insert_with(|| std::env::temp_dir().join(format!("sisu-tests-{}", std::process::id())))
            .clone();
    let _ = std::fs::create_dir_all(&dir);
    let exe = dir.join(format!("tests-{}", b.1.len()));
    let prog = match with_dispatcher(prog, options.case_seed, options.sim.is_some()) {
        Ok(p) => p,
        Err(what) => {
            host.stderr(&text(msg::NATIVE_TEST_UNSUPPORTED, &[("what", what)]));
            return Err(1);
        }
    };
    native::build(native::Code::Sir(&prog), &exe, profile, host)?;
    b.1.push((key, exe.clone()));
    Ok(exe)
}

/// One test's run: its result, what it wrote, and the case a property
/// recorded.
type Done = (TestResult, String, String, Option<Vec<i64>>, Vec<(String, String)>, Vec<sisu_interp::Choice>);

/// One test's run.
fn run_one(exe: &Path, k: usize, name: &str, options: &Options, real: bool) -> Done {
    let report = exe.with_file_name(format!("report-{}-{k}", std::process::id()));
    let _ = std::fs::remove_file(&report);
    let mut cmd = std::process::Command::new(exe);
    cmd.env("SISU_TEST_ENTRY", k.to_string()).env("SISU_TEST_REPORT", &report);
    if options.seed != 0 {
        cmd.env("SISU_SCHEDULE_SEED", options.seed.to_string());
    }
    // The interpreter's other schedulers (decision 501) and the `sim`
    // profile's faults (decision 502).
    match &options.strategy {
        sisu_interp::Strategy::Random => {}
        sisu_interp::Strategy::Pct { depth } => {
            cmd.env("SISU_SCHEDULER", format!("pct:{depth}"));
        }
        sisu_interp::Strategy::Exhaustive { prefix } => {
            let shown: Vec<String> = prefix.iter().map(u32::to_string).collect();
            cmd.env("SISU_SCHEDULER", format!("exhaustive:{}", shown.join(".")));
        }
    }
    if let Some(rate) = options.sim.filter(|r| *r > 0) {
        cmd.env("SISU_SIM_FAULTS", rate.to_string());
    }
    // The test's name and its stored snapshots (decision 461).
    cmd.env("SISU_TEST_NAME", name);
    if options.update {
        cmd.env("SISU_TEST_UPDATE", "1");
    }
    let stored = exe.with_file_name(format!("snapshots-{}-{k}", std::process::id()));
    let mut text = String::new();
    for (key, value) in options.snapshots.iter() {
        if key == name || key.starts_with(&format!("{name} #")) {
            text.push_str(&format!("{key}\0{value}\0"));
        }
    }
    if !text.is_empty() && std::fs::write(&stored, text).is_ok() {
        cmd.env("SISU_TEST_SNAPSHOTS", &stored);
    }
    // What std's `check` reads (decision 461): the seed of its cases and
    // the case a replay token names.
    cmd.env("SISU_TEST_CASE_SEED", options.case_seed.to_string());
    if let Some(choices) = &options.replay_case {
        let shown: Vec<String> = choices.iter().map(i64::to_string).collect();
        cmd.env("SISU_TEST_REPLAY_CASE", shown.join("."));
    }
    if !real {
        cmd.stdin(std::process::Stdio::null());
    }
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            let failed = TestResult::Failed(format!("{}: {e}\n", exe.display()));
            return (failed, String::new(), String::new(), None, Vec::new(), Vec::new());
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let written = std::fs::read_to_string(&report).ok();
    let _ = std::fs::remove_file(&report);
    // The failing case a property recorded, for the token.
    let mut case_file = report.clone().into_os_string();
    case_file.push(".case");
    let case = std::fs::read_to_string(&case_file).ok().map(|t| {
        t.split('.').filter(|x| !x.is_empty()).filter_map(|x| x.parse::<i64>().ok()).collect::<Vec<i64>>()
    });
    let _ = std::fs::remove_file(&case_file);
    let _ = std::fs::remove_file(&stored);
    // The exhaustive search's choices, `taken/options/y` dotted.
    let mut trail_file = report.clone().into_os_string();
    trail_file.push(".trail");
    let trail = std::fs::read_to_string(&trail_file).unwrap_or_default();
    let _ = std::fs::remove_file(&trail_file);
    let trail: Vec<sisu_interp::Choice> = trail
        .split('.')
        .filter_map(|c| {
            let mut parts = c.split('/');
            let taken = parts.next()?.parse().ok()?;
            let options = parts.next()?.parse().ok()?;
            Some(sisu_interp::Choice { taken, options, at_yield: parts.next()? == "y" })
        })
        .collect();
    // The snapshots it took, for `--update`.
    let mut snap_file = report.clone().into_os_string();
    snap_file.push(".snap");
    let taken = std::fs::read_to_string(&snap_file).unwrap_or_default();
    let _ = std::fs::remove_file(&snap_file);
    let parts: Vec<&str> = taken.split('\0').collect();
    let snapshots: Vec<(String, String)> =
        parts.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0].to_string(), c[1].to_string())).collect();
    let result = match written.as_deref().and_then(|w| w.split_once('\n')) {
        Some(("blocked", hole)) => TestResult::Blocked(hole.to_string()),
        Some((_, failure)) => TestResult::Failed(failure.to_string()),
        None if out.status.success() => TestResult::Passed,
        // An abort's message is the failure, as the interpreter's is.
        None => TestResult::Failed(std::mem::take(&mut stderr)),
    };
    (result, stdout, stderr, case, snapshots, trail)
}

/// Runs the tests at these indices natively; see the module's comment.
pub(crate) fn run(
    prog: &sir::Program,
    host: &mut dyn Host,
    options: &Options,
    which: &[usize],
    profile: Profile,
) -> Result<Vec<(String, TestResult, RunReport)>, i32> {
    if let Some(what) = refused(options) {
        host.stderr(&text(msg::NATIVE_TEST_UNSUPPORTED, &[("what", what.to_string())]));
        return Err(1);
    }
    let exe = executable(prog, options, profile, host)?;
    let real = |k: usize| {
        options.sim.is_none()
            && prog
                .tests
                .get(k)
                .and_then(|(_, f)| prog.functions.iter().find(|x| x.name == *f))
                .is_some_and(sir::takes_real_system)
    };
    let done: Mutex<Vec<Option<Done>>> = Mutex::new(vec![None; which.len()]);
    let parallel: Vec<usize> = (0..which.len()).filter(|&i| !real(which[i])).collect();
    let jobs = if options.jobs == 0 { sisu_interp::parallel::DEFAULT_JOBS } else { options.jobs }
        .min(parallel.len())
        .max(1);
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| {
                loop {
                    let at = next.fetch_add(1, Ordering::SeqCst);
                    let Some(&i) = parallel.get(at) else { break };
                    let name = prog.tests.get(which[i]).map_or("", |t| t.0.as_str());
                    let r = run_one(&exe, which[i], name, options, false);
                    done.lock().unwrap_or_else(|p| p.into_inner())[i] = Some(r);
                }
            });
        }
    });
    let mut done = done.into_inner().unwrap_or_else(|p| p.into_inner());
    for (i, &k) in which.iter().enumerate() {
        if done[i].is_none() {
            let name = prog.tests.get(k).map_or("", |t| t.0.as_str());
            done[i] = Some(run_one(&exe, k, name, options, true));
        }
    }
    let mut out = Vec::new();
    for (i, &k) in which.iter().enumerate() {
        let Some((r, o, e, case, snapshots, trail)) = done[i].take() else { continue };
        let Some((name, _)) = prog.tests.get(k) else { continue };
        let status = if matches!(r, TestResult::Passed) { 0 } else { 1 };
        out.push((
            name.clone(),
            r,
            RunReport { status, out: o, err: e, case, snapshots, trail, ..Default::default() },
        ));
    }
    Ok(out)
}
