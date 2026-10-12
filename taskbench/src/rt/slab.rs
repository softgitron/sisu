//! A minimal slab: stable indices, O(1) insert and remove, a free list.

/// Values stored at stable indices.
#[derive(Debug)]
pub struct Slab<T> {
    entries: Vec<Option<T>>,
    free: Vec<usize>,
    len: usize,
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Slab { entries: Vec::new(), free: Vec::new(), len: 0 }
    }
}

impl<T> Slab<T> {
    pub fn new() -> Slab<T> {
        Slab::default()
    }

    /// Stores `v` and returns its index.
    pub fn insert(&mut self, v: T) -> usize {
        self.len += 1;
        if let Some(i) = self.free.pop() {
            self.entries[i] = Some(v);
            i
        } else {
            self.entries.push(Some(v));
            self.entries.len() - 1
        }
    }

    /// Removes and returns the value at `i`, if any.
    pub fn remove(&mut self, i: usize) -> Option<T> {
        let v = self.entries.get_mut(i)?.take()?;
        self.free.push(i);
        self.len -= 1;
        Some(v)
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        self.entries.get(i)?.as_ref()
    }

    #[allow(dead_code)] // used by tests and kept for debugging
    pub fn len(&self) -> usize {
        self.len
    }

    #[allow(dead_code)] // as `len`
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().filter_map(|e| e.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::Slab;

    #[test]
    fn insert_remove_reuse() {
        let mut s = Slab::new();
        let a = s.insert("a");
        let b = s.insert("b");
        assert_eq!(s.len(), 2);
        assert_eq!(s.remove(a), Some("a"));
        assert_eq!(s.remove(a), None);
        let c = s.insert("c");
        assert_eq!(c, a, "freed index reused");
        assert_eq!(s.get(b), Some(&"b"));
        assert_eq!(s.iter().count(), 2);
    }
}
