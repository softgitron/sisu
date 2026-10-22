//! A fixed-size set of small integers, the dataflow lattice element, and
//! a table of such sets in one buffer.

#[derive(Debug, PartialEq, Eq, Hash, Default)]
pub struct BitSet {
    words: Vec<u64>,
}

impl Clone for BitSet {
    fn clone(&self) -> BitSet {
        BitSet { words: self.words.clone() }
    }

    /// Copies into the buffer already there: the dataflow copies states
    /// at every statement.
    fn clone_from(&mut self, o: &BitSet) {
        self.words.clone_from(&o.words);
    }
}

impl BitSet {
    pub fn new(n: usize) -> BitSet {
        BitSet { words: vec![0; n.div_ceil(64)] }
    }

    pub fn contains(&self, i: usize) -> bool {
        self.words.get(i / 64).is_some_and(|w| w & (1 << (i % 64)) != 0)
    }

    pub fn insert(&mut self, i: usize) -> bool {
        let w = i / 64;
        if w >= self.words.len() {
            self.words.resize(w + 1, 0);
        }
        let had = self.words[w] & (1 << (i % 64)) != 0;
        self.words[w] |= 1 << (i % 64);
        !had
    }

    pub fn remove(&mut self, i: usize) {
        if let Some(w) = self.words.get_mut(i / 64) {
            *w &= !(1 << (i % 64));
        }
    }

    /// Adds every member of `other`; returns whether anything changed.
    pub fn union(&mut self, other: &BitSet) -> bool {
        if other.words.len() > self.words.len() {
            self.words.resize(other.words.len(), 0);
        }
        let mut changed = false;
        for (a, b) in self.words.iter_mut().zip(&other.words) {
            let n = *a | b;
            changed |= n != *a;
            *a = n;
        }
        changed
    }

    /// Keeps only the members of `other`; returns whether anything changed.
    pub fn intersect(&mut self, other: &BitSet) -> bool {
        let mut changed = false;
        for (i, a) in self.words.iter_mut().enumerate() {
            let b = other.words.get(i).copied().unwrap_or(0);
            let n = *a & b;
            changed |= n != *a;
            *a = n;
        }
        changed
    }

    pub fn subtract(&mut self, other: &BitSet) {
        for (a, b) in self.words.iter_mut().zip(&other.words) {
            *a &= !b;
        }
    }

    pub fn clear(&mut self) {
        for w in &mut self.words {
            *w = 0;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|w| *w == 0)
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.words
            .iter()
            .enumerate()
            .flat_map(|(wi, w)| (0..64).filter(move |b| w & (1u64 << b) != 0).map(move |b| wi * 64 + b))
    }

    pub fn is_subset(&self, other: &BitSet) -> bool {
        self.words.iter().enumerate().all(|(i, a)| a & !other.words.get(i).copied().unwrap_or(0) == 0)
    }
}

/// One set per row, all rows the same number of words and kept in one
/// buffer: the loans dataflow's state has a set per local, and a copy of
/// it is one allocation rather than one per local. A row may be left out
/// of the buffer (a local whose type can hold no loan, decision 219): it
/// reads as empty, and nothing is written to it.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Rows {
    /// Words per row.
    width: usize,
    rows: usize,
    /// Where each row is in the buffer, by its place among the rows kept;
    /// [`LEFT_OUT`] for a row left out.
    slots: std::sync::Arc<[u32]>,
    words: Vec<u64>,
}

/// The slot of a row left out of the buffer.
pub const LEFT_OUT: u32 = u32::MAX;

impl Clone for Rows {
    fn clone(&self) -> Rows {
        Rows { width: self.width, rows: self.rows, slots: self.slots.clone(), words: self.words.clone() }
    }

    fn clone_from(&mut self, o: &Rows) {
        self.width = o.width;
        self.rows = o.rows;
        if !std::sync::Arc::ptr_eq(&self.slots, &o.slots) {
            self.slots = o.slots.clone();
        }
        self.words.clone_from(&o.words);
    }
}

impl Rows {
    /// `rows` empty sets, each with room for the members below `bits`.
    pub fn new(rows: usize, bits: usize) -> Rows {
        let slots: Vec<u32> = (0..rows as u32).collect();
        Rows::with_slots(slots.into(), bits)
    }

    /// Empty sets for the rows `slots` keeps (each its place in the
    /// buffer, or [`LEFT_OUT`]), each with room for the members below
    /// `bits`.
    pub fn with_slots(slots: std::sync::Arc<[u32]>, bits: usize) -> Rows {
        let width = bits.div_ceil(64);
        let kept = slots.iter().filter(|s| **s != LEFT_OUT).count();
        Rows { width, rows: slots.len(), slots, words: vec![0; kept * width] }
    }

    pub fn len(&self) -> usize {
        self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    /// Where row `r` is in the buffer, if it is kept.
    fn slot(&self, r: usize) -> Option<usize> {
        self.slots.get(r).filter(|s| **s != LEFT_OUT).map(|s| *s as usize)
    }

    /// The words of row `r`; none for a row left out.
    fn row(&self, r: usize) -> &[u64] {
        match self.slot(r) {
            Some(s) => self.words.get(s * self.width..(s + 1) * self.width).unwrap_or(&[]),
            None => &[],
        }
    }

    fn row_mut(&mut self, r: usize) -> &mut [u64] {
        let w = self.width;
        match self.slot(r) {
            Some(s) => self.words.get_mut(s * w..(s + 1) * w).unwrap_or(&mut []),
            None => &mut [],
        }
    }

    /// The words row `r` reads as: a row left out, or past the end, as
    /// many zero words as a row has.
    fn words_of(&self, r: usize) -> usize {
        if r < self.rows { self.width } else { 0 }
    }

    /// Makes every row `width` words wide (a member past the room made
    /// at the start; the loans of a function are known before its
    /// dataflow, so this does not happen in practice).
    fn widen(&mut self, width: usize) {
        if width <= self.width {
            return;
        }
        let kept = self.slots.iter().filter(|s| **s != LEFT_OUT).count();
        let mut words = vec![0; kept * width];
        for k in 0..kept {
            let old = self.words.get(k * self.width..(k + 1) * self.width).unwrap_or(&[]);
            words[k * width..k * width + old.len()].copy_from_slice(old);
        }
        self.width = width;
        self.words = words;
    }

    pub fn contains(&self, r: usize, i: usize) -> bool {
        self.row(r).get(i / 64).is_some_and(|w| w & (1 << (i % 64)) != 0)
    }

    pub fn insert(&mut self, r: usize, i: usize) -> bool {
        if self.slot(r).is_none() {
            return false;
        }
        self.widen(i / 64 + 1);
        let w = &mut self.row_mut(r)[i / 64];
        let had = *w & (1 << (i % 64)) != 0;
        *w |= 1 << (i % 64);
        !had
    }

    /// Row `r` as a set of its own.
    pub fn get(&self, r: usize) -> BitSet {
        match self.slot(r) {
            Some(_) => BitSet { words: self.row(r).to_vec() },
            None => BitSet { words: vec![0; self.words_of(r)] },
        }
    }

    /// Replaces row `r` with `b`.
    pub fn set(&mut self, r: usize, b: &BitSet) {
        self.clear_row(r);
        self.union_row(r, b);
    }

    /// Adds every member of `b` to row `r`; returns whether anything
    /// changed.
    pub fn union_row(&mut self, r: usize, b: &BitSet) -> bool {
        if self.slot(r).is_none() {
            return false;
        }
        if let Some(last) = b.words.iter().rposition(|w| *w != 0) {
            self.widen(last + 1);
        }
        let mut changed = false;
        for (a, b) in self.row_mut(r).iter_mut().zip(&b.words) {
            let n = *a | b;
            changed |= n != *a;
            *a = n;
        }
        changed
    }

    pub fn clear_row(&mut self, r: usize) {
        for w in self.row_mut(r) {
            *w = 0;
        }
    }

    /// Adds the members of row `r` to `out`.
    pub fn add_to(&self, r: usize, out: &mut BitSet) {
        let n = self.words_of(r);
        if n > out.words.len() {
            out.words.resize(n, 0);
        }
        for (a, b) in out.words.iter_mut().zip(self.row(r)) {
            *a |= b;
        }
    }

    pub fn iter_row(&self, r: usize) -> impl Iterator<Item = usize> + '_ {
        self.row(r)
            .iter()
            .enumerate()
            .flat_map(|(wi, w)| (0..64).filter(move |b| w & (1u64 << b) != 0).map(move |b| wi * 64 + b))
    }

    /// Adds every row of `o`, which keeps the same rows, to the same row
    /// here; returns whether anything changed.
    pub fn union(&mut self, o: &Rows) -> bool {
        self.widen(o.width);
        let mut changed = false;
        if self.width == o.width && self.words.len() == o.words.len() {
            for (a, b) in self.words.iter_mut().zip(&o.words) {
                let n = *a | b;
                changed |= n != *a;
                *a = n;
            }
            return changed;
        }
        for r in 0..self.rows.min(o.rows) {
            let theirs = o.row(r).to_vec();
            for (a, b) in self.row_mut(r).iter_mut().zip(&theirs) {
                let n = *a | b;
                changed |= n != *a;
                *a = n;
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_operations() {
        let mut a = BitSet::new(10);
        assert!(a.insert(3));
        assert!(!a.insert(3));
        a.insert(70);
        assert!(a.contains(70) && a.contains(3) && !a.contains(4));
        let mut b = BitSet::new(10);
        b.insert(3);
        assert!(b.is_subset(&a) && !a.is_subset(&b));
        assert!(a.intersect(&b));
        assert_eq!(a.iter().collect::<Vec<_>>(), vec![3]);
        a.remove(3);
        assert!(a.is_empty());
    }

    #[test]
    fn rows_are_sets() {
        let mut t = Rows::new(3, 10);
        assert!(t.insert(1, 3));
        assert!(!t.insert(1, 3));
        // A member past the room made at the start widens every row.
        assert!(t.insert(2, 70));
        assert!(t.contains(1, 3) && t.contains(2, 70) && !t.contains(0, 3));
        let mut b = BitSet::new(10);
        b.insert(5);
        assert!(t.union_row(0, &b));
        assert!(!t.union_row(0, &b));
        assert_eq!(t.iter_row(0).collect::<Vec<_>>(), vec![5]);
        let mut u = Rows::new(3, 10);
        u.insert(0, 6);
        assert!(t.union(&u));
        assert!(!t.union(&u));
        assert_eq!(t.get(0).iter().collect::<Vec<_>>(), vec![5, 6]);
        let mut out = BitSet::new(10);
        t.add_to(2, &mut out);
        assert_eq!(out.iter().collect::<Vec<_>>(), vec![70]);
        t.set(2, &b);
        assert_eq!(t.iter_row(2).collect::<Vec<_>>(), vec![5]);
        t.clear_row(2);
        assert_eq!(t.iter_row(2).count(), 0);
        let mut c = Rows::new(1, 1);
        c.clone_from(&t);
        assert_eq!(c, t);
    }
}
