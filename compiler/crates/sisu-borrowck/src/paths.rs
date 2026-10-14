//! Move paths: the places whose initialisation the dataflow tracks. A
//! path is a local followed by field and variant projections (`_3`,
//! `_3.1`, `_3.as(0).0`); a place through a reference, a list index or a
//! dict key is tracked as the path before that projection, since nothing
//! can be moved out of it. Paths form a tree per local.

use sisu_sir::{Function, Operand, Place, Proj, Rvalue, StatementKind};

pub type PathId = usize;

#[derive(Clone, Debug)]
pub struct PathData {
    pub place: Place,
    pub parent: Option<PathId>,
    pub children: Vec<PathId>,
}

pub struct MovePaths {
    pub paths: Vec<PathData>,
    /// The root path of each local.
    pub roots: Vec<PathId>,
}

/// The longest prefix of a place made of field and variant projections.
fn trackable(p: &Place) -> Place {
    let n =
        p.proj.iter().position(|x| !matches!(x, Proj::Field(_) | Proj::Downcast(_))).unwrap_or(p.proj.len());
    // A path never ends in a bare downcast: `_3.as(0)` is the value `_3`.
    let mut n = n;
    while n > 0 && matches!(p.proj[n - 1], Proj::Downcast(_)) {
        n -= 1;
    }
    Place { local: p.local, proj: p.proj[..n].to_vec() }
}

impl MovePaths {
    /// The paths of a function: every place moved, assigned or dropped,
    /// and a path per variant of every place whose discriminant is read
    /// (so a switch can mark the payloads of the other variants empty).
    pub fn build_with(f: &Function, types: &crate::types::Types) -> MovePaths {
        let mut mp = MovePaths::build(f);
        for b in &f.blocks {
            for s in &b.stmts {
                if let StatementKind::Assign(_, Rvalue::Discriminant(p)) = &s.kind {
                    let t = trackable(p);
                    if t.proj.len() != p.proj.len() {
                        continue;
                    }
                    let Some(ty) = types.place_ty(f, p) else { continue };
                    let Some(n) = types.variant_count(&ty) else { continue };
                    let base = mp.add(&t);
                    for v in 0..n as u32 {
                        let mut proj = t.proj.clone();
                        proj.push(Proj::Downcast(v));
                        let place = Place { local: t.local, proj };
                        if !mp.paths[base].children.iter().any(|c| mp.paths[*c].place == place) {
                            let id = mp.paths.len();
                            mp.paths.push(PathData { place, parent: Some(base), children: Vec::new() });
                            mp.paths[base].children.push(id);
                        }
                    }
                }
            }
        }
        mp
    }

    pub fn build(f: &Function) -> MovePaths {
        let mut mp = MovePaths { paths: Vec::new(), roots: Vec::new() };
        for l in 0..f.locals.len() {
            let id = mp.paths.len();
            mp.paths.push(PathData {
                place: Place::local(sisu_sir::LocalId(l as u32)),
                parent: None,
                children: Vec::new(),
            });
            mp.roots.push(id);
        }
        for b in &f.blocks {
            for s in &b.stmts {
                match &s.kind {
                    StatementKind::Assign(dest, rv) => {
                        mp.add(&trackable(dest));
                        if let Rvalue::Use(Operand::Move(p)) = rv {
                            mp.add(&trackable(p));
                        }
                        if let Rvalue::Aggregate(_, ops) | Rvalue::Closure(_, ops) = rv {
                            for o in ops {
                                if let Operand::Move(p) = o {
                                    mp.add(&trackable(p));
                                }
                            }
                        }
                        if let Rvalue::Binary(_, a, b) = rv {
                            for o in [a, b] {
                                if let Operand::Move(p) = o {
                                    mp.add(&trackable(p));
                                }
                            }
                        }
                        if let Rvalue::Unary(_, Operand::Move(p)) = rv {
                            mp.add(&trackable(p));
                        }
                    }
                    StatementKind::Call { dest, callee, args } => {
                        mp.add(&trackable(dest));
                        for o in args {
                            if let Operand::Move(p) = o {
                                mp.add(&trackable(p));
                            }
                        }
                        if let sisu_sir::Callee::Value(Operand::Move(p)) = callee {
                            mp.add(&trackable(p));
                        }
                    }
                    StatementKind::Drop { place, .. } => {
                        mp.add(&trackable(place));
                    }
                    StatementKind::StorageDead(_) => {}
                }
            }
        }
        mp
    }

    /// Adds the path for a place (and its prefixes), returning it.
    fn add(&mut self, p: &Place) -> PathId {
        let mut cur = self.roots[p.local.0 as usize];
        for i in 0..p.proj.len() {
            if matches!(p.proj[i], Proj::Downcast(_)) && i + 1 == p.proj.len() {
                break;
            }
            let prefix = Place { local: p.local, proj: p.proj[..=i].to_vec() };
            let found = self.paths[cur].children.iter().copied().find(|c| self.paths[*c].place == prefix);
            cur = match found {
                Some(c) => c,
                None => {
                    let id = self.paths.len();
                    self.paths.push(PathData { place: prefix, parent: Some(cur), children: Vec::new() });
                    self.paths[cur].children.push(id);
                    id
                }
            };
        }
        cur
    }

    /// The deepest path that is a prefix of `p`, and how much of `p` it
    /// covers: `Exact` when it is the whole place.
    pub fn lookup(&self, p: &Place) -> (PathId, Cover) {
        let Some(&root) = self.roots.get(p.local.0 as usize) else { return (0, Cover::Field) };
        let mut cur = root;
        for (i, proj) in p.proj.iter().enumerate() {
            match proj {
                Proj::Field(_) | Proj::Downcast(_) => {
                    let prefix = &p.proj[..=i];
                    match self.paths[cur]
                        .children
                        .iter()
                        .copied()
                        .find(|c| self.paths[*c].place.proj == prefix)
                    {
                        Some(c) => cur = c,
                        None => {
                            let trailing_downcast =
                                i + 1 == p.proj.len() && matches!(proj, Proj::Downcast(_));
                            return (cur, if trailing_downcast { Cover::Exact } else { Cover::Field });
                        }
                    }
                }
                _ => return (cur, Cover::Through),
            }
        }
        (cur, Cover::Exact)
    }

    /// The path and every path below it.
    pub fn descendants(&self, id: PathId) -> Vec<PathId> {
        let mut out = vec![id];
        let mut i = 0;
        while i < out.len() {
            out.extend(self.paths[out[i]].children.iter().copied());
            i += 1;
        }
        out
    }

    pub fn ancestors(&self, id: PathId) -> Vec<PathId> {
        let mut out = Vec::new();
        let mut cur = self.paths[id].parent;
        while let Some(p) = cur {
            out.push(p);
            cur = self.paths[p].parent;
        }
        out
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn local_of(&self, id: PathId) -> sisu_sir::LocalId {
        self.paths[id].place.local
    }
}

/// How much of a place its path covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cover {
    /// The place is the path.
    Exact,
    /// The place continues through a reference, index or key of the path's
    /// value: using it uses the whole value.
    Through,
    /// The place is a part of the path's value that has no path of its own.
    Field,
}
