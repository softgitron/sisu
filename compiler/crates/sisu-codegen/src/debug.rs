//! The debug information of an executable (10, "Debugging and profiling":
//! "DWARF output with full type information"; decision 227): DWARF 4,
//! written with gimli into the object file beside the code.
//!
//! - A line table maps each function's machine code to the Sisu file and
//!   line of the SIR statement it was compiled from (Cranelift carries the
//!   location of each statement to the instructions made for it, as a
//!   [`crate::func::srcloc`]), so a debugger's breakpoints, steps and
//!   backtraces name Sisu lines.
//! - A subprogram per function instance names it, with its named locals
//!   and parameters, each where its stack slot is (a parameter through the
//!   slot that keeps its address), and of its Sisu type.
//! - Call frame information (`.debug_frame`), from Cranelift's unwind
//!   information, lets a debugger unwind through Sisu frames.
//!
//! The unit says its language is Rust, the closest one debuggers know: a
//! text is described as Rust's `&str` (`data_ptr` and `length`), so they
//! print it as text; a list's elements as a slice; an enum, an option, a
//! result and an error set's variant as a variant part, so they print the
//! variant the value holds.

use crate::Error;
use crate::layout::Types;
use cranelift_codegen::ir::StackSlot;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::isa::unwind::UnwindInfo;
use cranelift_module::FuncId;
use cranelift_object::ObjectProduct;
use cranelift_object::object::write::{Relocation as ObjReloc, SectionId as ObjSection};
use cranelift_object::object::{RelocationEncoding, RelocationFlags, RelocationKind, SectionKind};
use gimli::write::{
    Address, AttributeValue, DwarfUnit, EndianVec, Expression, FileId, FrameTable, LineProgram, LineString,
    Range, RangeList, RelocateWriter, Relocation, RelocationTarget, Sections, UnitEntryId,
};
use gimli::{Encoding, Format, LineEncoding, LittleEndian, Register, SectionId, constants};
use sisu_rt::abi;
use sisu_sir as sir;
use sisu_sir::Type;
use std::collections::HashMap;

/// A named local of a function, as the debug information describes it.
pub(crate) struct Var {
    pub name: String,
    pub ty: Type,
    pub slot: StackSlot,
    /// The slot holds the address of the value (a parameter's, whose
    /// memory the caller provides), not the value.
    pub indirect: bool,
    /// A parameter of the function (the environment of a closure is its
    /// first).
    pub param: bool,
    /// Where it is declared.
    pub line: Option<u32>,
}

/// What the debug information says of one compiled function.
pub(crate) struct FnDebug {
    pub id: FuncId,
    /// The name a debugger shows: the SIR function's, or what the code
    /// generator made it for.
    pub name: String,
    /// The source the function comes from, and its line, for one compiled
    /// from SIR.
    pub source: Option<u32>,
    pub line: u32,
    pub vars: Vec<Var>,
    /// From compiling it: the size of its code, the source location of
    /// each run of it (start, end, [`crate::func::srcloc`] bits), each stack
    /// slot's offset from the stack pointer below the frame, and its unwind
    /// information.
    pub size: u32,
    pub lines: Vec<(u32, u32, u32)>,
    pub slots: Vec<u32>,
    pub unwind: Option<UnwindInfo>,
}

impl FnDebug {
    /// A function named `name`, with no source.
    pub(crate) fn new(id: FuncId, name: String) -> FnDebug {
        FnDebug {
            id,
            name,
            source: None,
            line: 0,
            vars: Vec::new(),
            size: 0,
            lines: Vec::new(),
            slots: Vec::new(),
            unwind: None,
        }
    }

    /// With what compiling the function gave: the size of its code and the
    /// rest.
    pub(crate) fn compiled(mut self, size: u32, c: crate::workers::Compiled) -> FnDebug {
        self.size = size;
        self.lines = c.lines;
        self.slots = c.slots;
        self.unwind = c.unwind;
        self
    }
}

/// Where the debug information says the program's sources are.
#[derive(Clone, Debug, Default)]
pub struct Sources {
    /// The directory the program's source paths are relative to.
    pub comp_dir: String,
    /// The directory holding the standard library's sources, whose paths
    /// are `<std>/NAME.sisu` in SIR.
    pub std_dir: Option<String>,
}

/// A section's bytes and the relocations its addresses and offsets need.
#[derive(Clone)]
struct SectionWriter {
    data: EndianVec<LittleEndian>,
    relocs: Vec<Relocation>,
}

impl RelocateWriter for SectionWriter {
    type Writer = EndianVec<LittleEndian>;

    fn writer(&self) -> &Self::Writer {
        &self.data
    }

    fn writer_mut(&mut self) -> &mut Self::Writer {
        &mut self.data
    }

    fn relocate(&mut self, relocation: Relocation) {
        self.relocs.push(relocation);
    }
}

fn dwarf_error(e: gimli::write::Error) -> Error {
    Error::internal(format!("writing the debug information: {e}"))
}

/// The stack pointer's DWARF register on this machine.
fn stack_pointer() -> Register {
    if cfg!(target_arch = "aarch64") { Register(31) } else { Register(7) }
}

/// Writes the debug information of the compiled functions `fns` into the
/// object.
pub(crate) fn write(
    product: &mut ObjectProduct,
    prog: &sir::Program,
    fns: &[FnDebug],
    types: &mut Types<'_>,
    sources: &Sources,
    isa: &dyn TargetIsa,
) -> Result<(), Error> {
    let encoding = Encoding { format: Format::Dwarf32, version: 4, address_size: 8 };
    let mut dwarf = DwarfUnit::new(encoding);
    // A name of a directory or a file is never empty, nor holds a null
    // byte (gimli's rule).
    let clean = |s: &str, empty: &str| {
        let s: String = s.chars().filter(|c| *c != '\0').collect();
        if s.is_empty() { empty.to_string() } else { s }
    };
    let comp_dir_text = clean(&sources.comp_dir, ".");
    let comp_dir = LineString::new(comp_dir_text.as_bytes(), encoding, &mut dwarf.line_strings);
    let main_name = clean(prog.sources.first().map(String::as_str).unwrap_or(""), "<program>");
    let main_file = LineString::new(main_name.as_bytes(), encoding, &mut dwarf.line_strings);
    dwarf.unit.line_program =
        LineProgram::new(encoding, LineEncoding::default(), comp_dir, None, main_file, None);
    // The files of the program's sources: a path relative to the unit's
    // directory, or one of the standard library's in its directory.
    let here = dwarf.unit.line_program.default_directory();
    let std_dir = sources.std_dir.as_ref().map(|d| clean(d, ".")).map(|d| {
        let s = LineString::new(d.as_bytes(), encoding, &mut dwarf.line_strings);
        dwarf.unit.line_program.add_directory(s)
    });
    let mut files: Vec<FileId> = Vec::new();
    for s in &prog.sources {
        let (dir, name) = match (s.strip_prefix("<std>/"), std_dir) {
            (Some(rest), Some(d)) => (d, rest),
            _ => (here, s.as_str()),
        };
        let name = clean(name, "<source>");
        let name = LineString::new(name.as_bytes(), encoding, &mut dwarf.line_strings);
        files.push(dwarf.unit.line_program.add_file(name, dir, None));
    }
    // Each function's code, by its symbol: its rows.
    for (k, f) in fns.iter().enumerate() {
        let Some(file) = f.source.and_then(|s| files.get(s as usize).copied()) else { continue };
        let program = &mut dwarf.unit.line_program;
        program.begin_sequence(Some(Address::Symbol { symbol: k, addend: 0 }));
        // The code before the first statement's (the prologue, and the
        // keeping of the parameters' addresses) is the declaration's line;
        // the body begins at the first statement's code, where a debugger
        // stops for a breakpoint on the function.
        let first = f.lines.iter().map(|l| l.2).find(|&loc| loc != u32::MAX);
        let row = program.row();
        row.address_offset = 0;
        row.file = file;
        row.line = u64::from(if f.line > 0 { f.line } else { first.map_or(0, |loc| loc >> 12) });
        row.column = 0;
        program.generate_row();
        let mut last: Option<u32> = None;
        for &(start, _, loc) in &f.lines {
            if loc == u32::MAX || last == Some(loc) {
                continue;
            }
            let row = program.row();
            row.prologue_end = last.is_none();
            row.address_offset = u64::from(start);
            row.file = file;
            row.line = u64::from(loc >> 12);
            row.column = u64::from(loc & 0xFFF);
            program.generate_row();
            last = Some(loc);
        }
        program.end_sequence(u64::from(f.size));
    }
    // The unit, and its code's ranges.
    let ranges: Vec<Range> = fns
        .iter()
        .enumerate()
        .filter(|(_, f)| f.size > 0)
        .map(|(k, f)| Range::StartLength {
            begin: Address::Symbol { symbol: k, addend: 0 },
            length: u64::from(f.size),
        })
        .collect();
    let range_list = dwarf.unit.ranges.add(RangeList(ranges));
    let root = dwarf.unit.root();
    {
        let e = dwarf.unit.get_mut(root);
        e.set(constants::DW_AT_producer, AttributeValue::String(b"sisu native debug backend".to_vec()));
        e.set(constants::DW_AT_language, AttributeValue::Language(constants::DW_LANG_Rust));
        e.set(constants::DW_AT_name, AttributeValue::String(main_name.into_bytes()));
        e.set(constants::DW_AT_comp_dir, AttributeValue::String(comp_dir_text.as_bytes().to_vec()));
        e.set(constants::DW_AT_low_pc, AttributeValue::Address(Address::Constant(0)));
        e.set(constants::DW_AT_ranges, AttributeValue::RangeListRef(range_list));
    }
    let mut tb = TypeBuilder { types, ids: HashMap::new(), root, words: HashMap::new() };
    for (k, f) in fns.iter().enumerate() {
        let sub = dwarf.unit.add(root, constants::DW_TAG_subprogram);
        let e = dwarf.unit.get_mut(sub);
        e.set(constants::DW_AT_name, AttributeValue::String(f.name.as_bytes().to_vec()));
        e.set(constants::DW_AT_low_pc, AttributeValue::Address(Address::Symbol { symbol: k, addend: 0 }));
        e.set(constants::DW_AT_high_pc, AttributeValue::Udata(u64::from(f.size)));
        let mut base = Expression::new();
        base.op(constants::DW_OP_call_frame_cfa);
        e.set(constants::DW_AT_frame_base, AttributeValue::Exprloc(base));
        if let Some(file) = f.source.and_then(|s| files.get(s as usize).copied()) {
            e.set(constants::DW_AT_decl_file, AttributeValue::FileIndex(Some(file)));
            e.set(constants::DW_AT_decl_line, AttributeValue::Udata(u64::from(f.line)));
        }
        for v in &f.vars {
            let Some(&offset) = f.slots.get(v.slot.as_u32() as usize) else { continue };
            let ty = tb.ty(&mut dwarf, &v.ty);
            let tag = if v.param { constants::DW_TAG_formal_parameter } else { constants::DW_TAG_variable };
            let var = dwarf.unit.add(sub, tag);
            let e = dwarf.unit.get_mut(var);
            e.set(constants::DW_AT_name, AttributeValue::String(v.name.as_bytes().to_vec()));
            e.set(constants::DW_AT_type, AttributeValue::UnitRef(ty));
            let mut loc = Expression::new();
            loc.op_breg(stack_pointer(), i64::from(offset));
            if v.indirect {
                loc.op_deref();
            }
            e.set(constants::DW_AT_location, AttributeValue::Exprloc(loc));
            if let (Some(line), Some(file)) = (v.line, f.source.and_then(|s| files.get(s as usize).copied()))
            {
                e.set(constants::DW_AT_decl_file, AttributeValue::FileIndex(Some(file)));
                e.set(constants::DW_AT_decl_line, AttributeValue::Udata(u64::from(line)));
            }
        }
    }
    let mut sections =
        Sections::new(SectionWriter { data: EndianVec::new(LittleEndian), relocs: Vec::new() });
    dwarf.write(&mut sections).map_err(dwarf_error)?;
    // How to unwind each function.
    if let Some(cie) = isa.create_systemv_cie() {
        let mut frames = FrameTable::default();
        let cie = frames.add_cie(cie);
        for (k, f) in fns.iter().enumerate() {
            if let Some(UnwindInfo::SystemV(info)) = &f.unwind {
                frames.add_fde(cie, info.to_fde(Address::Symbol { symbol: k, addend: 0 }));
            }
        }
        frames.write_debug_frame(&mut sections.debug_frame).map_err(dwarf_error)?;
    }
    add_sections(product, fns, &sections)
}

/// Adds the sections the writer made to the object, with their
/// relocations: an address against its function's symbol, an offset into
/// another section against that section.
fn add_sections(
    product: &mut ObjectProduct,
    fns: &[FnDebug],
    sections: &Sections<SectionWriter>,
) -> Result<(), Error> {
    let mut ids: HashMap<SectionId, ObjSection> = HashMap::new();
    sections.for_each(|id, w| {
        if !w.data.slice().is_empty() {
            let s = product.object.add_section(Vec::new(), id.name().as_bytes().to_vec(), SectionKind::Debug);
            product.object.append_section_data(s, w.data.slice(), 1);
            ids.insert(id, s);
        }
        Ok::<(), Error>(())
    })?;
    let mut pending: Vec<(ObjSection, ObjReloc)> = Vec::new();
    sections.for_each(|id, w| {
        let Some(&at) = ids.get(&id) else { return Ok(()) };
        for r in &w.relocs {
            let symbol = match r.target {
                RelocationTarget::Symbol(k) => match fns.get(k) {
                    Some(f) => product.function_symbol(f.id),
                    None => return Err(Error::internal("a debug relocation to no function")),
                },
                RelocationTarget::Section(s) => match ids.get(&s) {
                    Some(&t) => product.object.section_symbol(t),
                    None => return Err(Error::internal("a debug relocation to a section not written")),
                },
            };
            let flags = RelocationFlags::Generic {
                kind: RelocationKind::Absolute,
                encoding: RelocationEncoding::Generic,
                size: r.size * 8,
            };
            pending.push((at, ObjReloc { offset: r.offset as u64, symbol, addend: r.addend, flags }));
        }
        Ok(())
    })?;
    for (at, r) in pending {
        product
            .object
            .add_relocation(at, r)
            .map_err(|e| Error::internal(format!("a relocation of the debug information: {e}")))?;
    }
    Ok(())
}

/// A variant's fields as the debug information describes them: each one's
/// name, type and offset in the value.
type Fields = Vec<(String, Type, u32)>;

/// The DWARF types of the program's types, made once each.
struct TypeBuilder<'t, 'p> {
    types: &'t mut Types<'p>,
    ids: HashMap<Type, UnitEntryId>,
    root: UnitEntryId,
    /// The base types of words, by name.
    words: HashMap<&'static str, UnitEntryId>,
}

impl TypeBuilder<'_, '_> {
    /// A base type of `size` bytes.
    fn base(
        &mut self,
        dwarf: &mut DwarfUnit,
        name: &'static str,
        ate: constants::DwAte,
        size: u32,
    ) -> UnitEntryId {
        if let Some(&id) = self.words.get(name) {
            return id;
        }
        let id = dwarf.unit.add(self.root, constants::DW_TAG_base_type);
        let e = dwarf.unit.get_mut(id);
        e.set(constants::DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
        e.set(constants::DW_AT_encoding, AttributeValue::Encoding(ate));
        e.set(constants::DW_AT_byte_size, AttributeValue::Udata(u64::from(size)));
        self.words.insert(name, id);
        id
    }

    fn word(&mut self, dwarf: &mut DwarfUnit) -> UnitEntryId {
        self.base(dwarf, "u64", constants::DW_ATE_unsigned, 8)
    }

    /// A pointer to `to` (or to nothing), named `name`.
    fn pointer(&mut self, dwarf: &mut DwarfUnit, name: &str, to: Option<UnitEntryId>) -> UnitEntryId {
        let id = dwarf.unit.add(self.root, constants::DW_TAG_pointer_type);
        let e = dwarf.unit.get_mut(id);
        e.set(constants::DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
        e.set(constants::DW_AT_byte_size, AttributeValue::Udata(8));
        if let Some(t) = to {
            e.set(constants::DW_AT_type, AttributeValue::UnitRef(t));
        }
        id
    }

    /// A struct of `size` bytes with no members yet.
    fn structure(&mut self, dwarf: &mut DwarfUnit, name: &str, size: u32) -> UnitEntryId {
        let id = dwarf.unit.add(self.root, constants::DW_TAG_structure_type);
        let e = dwarf.unit.get_mut(id);
        e.set(constants::DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
        e.set(constants::DW_AT_byte_size, AttributeValue::Udata(u64::from(size)));
        id
    }

    fn member(
        &mut self,
        dwarf: &mut DwarfUnit,
        parent: UnitEntryId,
        name: &str,
        ty: UnitEntryId,
        offset: u32,
    ) -> UnitEntryId {
        let id = dwarf.unit.add(parent, constants::DW_TAG_member);
        let e = dwarf.unit.get_mut(id);
        e.set(constants::DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
        e.set(constants::DW_AT_type, AttributeValue::UnitRef(ty));
        e.set(constants::DW_AT_data_member_location, AttributeValue::Udata(u64::from(offset)));
        id
    }

    /// A slice of `elem` as Rust's `&[T]` (`&str` for text): the address of
    /// the first element and the number of them.
    fn slice(&mut self, dwarf: &mut DwarfUnit, name: &str, elem: UnitEntryId) -> UnitEntryId {
        let id = self.structure(dwarf, name, 16);
        let p = self.pointer(dwarf, "*const", Some(elem));
        self.member(dwarf, id, "data_ptr", p, 0);
        let w = self.word(dwarf);
        self.member(dwarf, id, "length", w, 8);
        id
    }

    /// The words of a value whose parts the debug information does not
    /// describe.
    fn opaque(&mut self, dwarf: &mut DwarfUnit, name: &str, size: u32) -> UnitEntryId {
        let id = self.structure(dwarf, name, size);
        let w = self.word(dwarf);
        for k in 0..size / abi::WORD {
            self.member(dwarf, id, &format!("word{k}"), w, k * abi::WORD);
        }
        id
    }

    /// The DWARF type of a (concrete) SIR type.
    fn ty(&mut self, dwarf: &mut DwarfUnit, t: &Type) -> UnitEntryId {
        if let Some(&id) = self.ids.get(t) {
            return id;
        }
        let name = sir::write::ty(t);
        let size = self.types.size(t).unwrap_or(0);
        let id = match t {
            Type::Int => self.base(dwarf, "int", constants::DW_ATE_signed, 8),
            Type::Float => self.base(dwarf, "f64", constants::DW_ATE_float, 8),
            // A truth value is a word holding 0 or 1, read by its first byte.
            Type::Bool => self.base(dwarf, "bool", constants::DW_ATE_boolean, 1),
            Type::Char => self.base(dwarf, "char", constants::DW_ATE_UTF, 4),
            Type::Unit | Type::Never | Type::Const(_) | Type::ConstOp(..) => self.structure(dwarf, "()", 0),
            Type::Str => {
                let byte = self.base(dwarf, "u8", constants::DW_ATE_unsigned_char, 1);
                self.slice(dwarf, "&str", byte)
            }
            Type::Ref(inner, _) if crate::layout::fat(inner) => self.opaque(dwarf, &name, size),
            Type::Ref(inner, _) => {
                // A placeholder first: a type may reach itself through a
                // reference.
                let id = dwarf.unit.add(self.root, constants::DW_TAG_pointer_type);
                self.ids.insert(t.clone(), id);
                let to = self.ty(dwarf, inner);
                let e = dwarf.unit.get_mut(id);
                e.set(constants::DW_AT_name, AttributeValue::String(name.into_bytes()));
                e.set(constants::DW_AT_byte_size, AttributeValue::Udata(8));
                e.set(constants::DW_AT_type, AttributeValue::UnitRef(to));
                return id;
            }
            Type::Tuple(items) => {
                let id = self.structure(dwarf, &name, size);
                self.ids.insert(t.clone(), id);
                for (k, it) in items.iter().enumerate() {
                    if let Ok((off, _)) = self.types.field(t, None, k as u32) {
                        let ty = self.ty(dwarf, it);
                        self.member(dwarf, id, &format!("__{k}"), ty, off);
                    }
                }
                return id;
            }
            Type::Named(n, args) => return self.named(dwarf, t, n, args, &name, size),
            // An error set of one error is its value; of more, a variant
            // part chooses by the number of the error's own type, which the
            // value's first word holds.
            Type::ErrSet(names) if names.len() == 1 => {
                let id = self.ty(dwarf, &Type::Named(names[0].clone(), Vec::new()));
                self.ids.insert(t.clone(), id);
                return id;
            }
            Type::ErrSet(names) => {
                let id = self.structure(dwarf, &name, size);
                self.ids.insert(t.clone(), id);
                let w = self.word(dwarf);
                let part = dwarf.unit.add(id, constants::DW_TAG_variant_part);
                let discr = self.member(dwarf, part, "set", w, abi::ERROR_DECL);
                dwarf.unit.get_mut(discr).set(constants::DW_AT_artificial, AttributeValue::Flag(true));
                dwarf.unit.get_mut(part).set(constants::DW_AT_discr, AttributeValue::UnitRef(discr));
                for n in names {
                    let member = Type::Named(n.clone(), Vec::new());
                    // A member no value was made of is never the value's.
                    let Some(number) = self.types.known_id(&member) else { continue };
                    let variant = dwarf.unit.add(part, constants::DW_TAG_variant);
                    dwarf
                        .unit
                        .get_mut(variant)
                        .set(constants::DW_AT_discr_value, AttributeValue::Udata(u64::from(number)));
                    // The error's value as the one field of a struct named
                    // by it, so that a debugger prints `Problem(Missing)`.
                    let ty = self.ty(dwarf, &member);
                    let wrap = self.structure(dwarf, n, size);
                    self.member(dwarf, wrap, "__0", ty, 0);
                    self.member(dwarf, variant, n, wrap, 0);
                }
                return id;
            }
            _ => self.opaque(dwarf, &name, size),
        };
        self.ids.insert(t.clone(), id);
        id
    }

    fn named(
        &mut self,
        dwarf: &mut DwarfUnit,
        t: &Type,
        n: &str,
        args: &[Type],
        name: &str,
        size: u32,
    ) -> UnitEntryId {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Type::Unit);
        let simple = match n {
            "u8" | "u16" | "u32" | "u64" => Some(self.base(dwarf, "u64", constants::DW_ATE_unsigned, 8)),
            "i8" | "i16" | "i32" | "DateTime" => Some(self.base(dwarf, "i64", constants::DW_ATE_signed, 8)),
            "u128" => Some(self.base(dwarf, "u128", constants::DW_ATE_unsigned, 16)),
            "i128" => Some(self.base(dwarf, "i128", constants::DW_ATE_signed, 16)),
            "f32" => Some(self.base(dwarf, "f32", constants::DW_ATE_float, 4)),
            _ => None,
        };
        if let Some(id) = simple {
            self.ids.insert(t.clone(), id);
            return id;
        }
        let id = match n {
            // A text: its bytes as `&str`, and the capacity.
            "String" | "Path" | "bytes" => {
                let id = self.structure(dwarf, name, size);
                self.ids.insert(t.clone(), id);
                let byte = self.base(dwarf, "u8", constants::DW_ATE_unsigned_char, 1);
                let text = self.slice(dwarf, "&str", byte);
                self.member(dwarf, id, "text", text, 0);
                let w = self.word(dwarf);
                self.member(dwarf, id, "capacity", w, 16);
                id
            }
            // A list: its elements as a slice, and the capacity.
            "list" | "Array" | "Slice" => {
                let id = self.structure(dwarf, name, size);
                self.ids.insert(t.clone(), id);
                let elem = self.ty(dwarf, &arg(0));
                let items = self.slice(dwarf, &format!("&[{}]", sir::write::ty(&arg(0))), elem);
                self.member(dwarf, id, "items", items, 0);
                let w = self.word(dwarf);
                self.member(dwarf, id, "capacity", w, 16);
                id
            }
            "Box" => {
                let id = dwarf.unit.add(self.root, constants::DW_TAG_pointer_type);
                self.ids.insert(t.clone(), id);
                let to = self.ty(dwarf, &arg(0));
                let e = dwarf.unit.get_mut(id);
                e.set(constants::DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
                e.set(constants::DW_AT_byte_size, AttributeValue::Udata(8));
                e.set(constants::DW_AT_type, AttributeValue::UnitRef(to));
                id
            }
            "Option" | "Result" => {
                let names: &[&str] = if n == "Option" { &["Some", "None"] } else { &["Ok", "Err"] };
                let payloads =
                    if n == "Option" { vec![Some(arg(0)), None] } else { vec![Some(arg(0)), Some(arg(1))] };
                let variants: Vec<(String, Fields)> = names
                    .iter()
                    .zip(payloads)
                    .map(|(v, p)| {
                        let fields = p.map(|p| vec![("__0".to_string(), p, abi::WORD)]).unwrap_or_default();
                        (v.to_string(), fields)
                    })
                    .collect();
                self.enumeration(dwarf, t, name, size, 0, &[], variants)
            }
            _ => match self.types.decl(n).cloned() {
                Some(sir::TypeDecl::Struct { fields, .. }) => {
                    let id = self.structure(dwarf, name, size);
                    self.ids.insert(t.clone(), id);
                    for (k, f) in fields.iter().enumerate() {
                        if let Ok((off, ft)) = self.types.field(t, None, k as u32) {
                            let ty = self.ty(dwarf, &ft);
                            self.member(dwarf, id, &f.name, ty, off);
                        }
                    }
                    id
                }
                Some(sir::TypeDecl::Enum { variants, error, .. }) => {
                    let mut vs = Vec::new();
                    for (v, var) in variants.iter().enumerate() {
                        let mut fields = Vec::new();
                        for (k, (fname, _)) in var.fields.iter().enumerate() {
                            if let Ok((off, ft)) = self.types.field(t, Some(v as u32), k as u32) {
                                let fname = fname.clone().unwrap_or_else(|| format!("__{k}"));
                                fields.push((fname, ft, off));
                            }
                        }
                        vs.push((var.name.clone(), fields));
                    }
                    // An error value's own type and its return trace come
                    // first.
                    let (tag, head): (u32, &[(&str, u32)]) = if error {
                        (abi::ERROR_TAG, &[("set", abi::ERROR_DECL), ("trace", abi::ERROR_TRACE)])
                    } else {
                        (0, &[])
                    };
                    self.enumeration(dwarf, t, name, size, tag, head, vs)
                }
                None => {
                    if size == abi::WORD {
                        let w = self.word(dwarf);
                        self.pointer(dwarf, name, Some(w))
                    } else {
                        self.opaque(dwarf, name, size)
                    }
                }
            },
        };
        self.ids.insert(t.clone(), id);
        id
    }

    /// An enum: a struct whose variant part chooses by the tag word at
    /// `tag` which variant's fields it holds, after the words `head`.
    #[allow(clippy::too_many_arguments)]
    fn enumeration(
        &mut self,
        dwarf: &mut DwarfUnit,
        t: &Type,
        name: &str,
        size: u32,
        tag: u32,
        head: &[(&str, u32)],
        variants: Vec<(String, Fields)>,
    ) -> UnitEntryId {
        let id = self.structure(dwarf, name, size);
        self.ids.insert(t.clone(), id);
        let w = self.word(dwarf);
        // Artificial, as the tag is: a debugger takes the first member that
        // is not for the variant the value holds.
        for (hname, off) in head {
            let m = self.member(dwarf, id, hname, w, *off);
            dwarf.unit.get_mut(m).set(constants::DW_AT_artificial, AttributeValue::Flag(true));
        }
        let part = dwarf.unit.add(id, constants::DW_TAG_variant_part);
        let discr = self.member(dwarf, part, "tag", w, tag);
        dwarf.unit.get_mut(discr).set(constants::DW_AT_artificial, AttributeValue::Flag(true));
        dwarf.unit.get_mut(part).set(constants::DW_AT_discr, AttributeValue::UnitRef(discr));
        for (v, (vname, fields)) in variants.into_iter().enumerate() {
            let variant = dwarf.unit.add(part, constants::DW_TAG_variant);
            dwarf.unit.get_mut(variant).set(constants::DW_AT_discr_value, AttributeValue::Udata(v as u64));
            // The variant's fields, as a struct named by the variant, which
            // covers the whole value (its fields at their own offsets).
            let vty = self.structure(dwarf, &vname, size);
            for (fname, ft, off) in fields {
                let ty = self.ty(dwarf, &ft);
                self.member(dwarf, vty, &fname, ty, off);
            }
            self.member(dwarf, variant, &vname, vty, 0);
        }
        id
    }
}
