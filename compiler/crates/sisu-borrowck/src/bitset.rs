//! A fixed-size set of small integers, the dataflow lattice element.

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BitSet {
    words: Vec<u64>,
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
}
