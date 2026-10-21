//! The formatter with the names it needs (10, "Formatter": `T ! E`, not
//! `Result[T, E]`). `sisu_fmt` works on the syntax alone, and whether `E`
//! is an error set is not in it: the driver resolves the file and tells
//! the formatter which written types name error sets.

use crate::db::{Checked, Db};
use sisu_resolve::{DefKind, ModuleId, Program};

/// Whether the type written `written` (`E`, `json.Error`) names an error
/// set where `module` sees it.
pub fn names_error_set(prog: &Program, module: ModuleId, written: &str) -> bool {
    let mut segments = written.split('.').map(str::trim);
    let Some(first) = segments.next() else { return false };
    let Some(mut d) = prog.lookup(module, first) else { return false };
    for s in segments {
        let DefKind::Module(m) = prog.def(d).kind else { return false };
        match prog.lookup_in(m, s) {
            Some(next) => d = next,
            None => return false,
        }
    }
    prog.is_error_set(d)
}

/// The canonical text of `text`, the file `name` of the program `c`
/// checked: `Result[T, E]` is written `T ! E` where `E` names an error
/// set in the file's module.
pub fn format_in(c: &Checked, name: &str, text: &str) -> Result<String, sisu_fmt::Refusal> {
    let prog = &c.program;
    let file = c.map.files().iter().find(|f| f.name == name).map(|f| f.id);
    let module = file
        .and_then(|f| prog.modules.iter().position(|m| m.file == f && !m.is_std))
        .map(|i| ModuleId(i as u32));
    match module {
        Some(m) => sisu_fmt::format_with(text, &|written| names_error_set(prog, m, written)),
        None => sisu_fmt::format(text),
    }
}

/// The canonical text of the file `name`, resolved as a program of its
/// own (a module of a package without the package's other modules, so a
/// name it imports from them is not known as an error set, and its
/// `Result[T, E]` stays as written). A file that writes no `Result[` is
/// not resolved at all.
pub fn format_file(name: &str, text: &str) -> Result<String, sisu_fmt::Refusal> {
    if !text.contains("Result[") {
        return sisu_fmt::format(text);
    }
    let mut db = Db::new();
    db.set_file(name, text);
    match db.checked(name) {
        Some(c) => format_in(&c, name, text),
        None => sisu_fmt::format(text),
    }
}
