//! Generates one `MsgId` constant per catalog entry, so code can name only
//! messages that exist. `lex.tab-in-indentation` becomes
//! `LEX_TAB_IN_INDENTATION`.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let catalog = PathBuf::from("../../catalog/messages.catalog");
    println!("cargo:rerun-if-changed={}", catalog.display());
    let text = fs::read_to_string(&catalog).unwrap_or_default();
    let mut out = String::new();
    let mut names = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let (Some(_code), Some(_kind), Some(id)) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        let name: String =
            id.chars().map(|c| if c == '.' || c == '-' { '_' } else { c.to_ascii_uppercase() }).collect();
        out.push_str(&format!("pub const {name}: MsgId = MsgId::new({id:?});\n"));
        names.push(name);
    }
    out.push_str("/// Every message identifier in the catalog.\npub const ALL: &[MsgId] = &[\n");
    for n in &names {
        out.push_str(&format!("    {n},\n"));
    }
    out.push_str("];\n");
    let dest = PathBuf::from(env::var("OUT_DIR").unwrap_or_else(|_| ".".into())).join("msg_generated.rs");
    if let Err(e) = fs::write(&dest, out) {
        println!("cargo:warning=cannot write {}: {e}", dest.display());
    }
}
