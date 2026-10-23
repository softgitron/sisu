//! A fast hasher for the code generator's maps (FxHash's multiply and
//! rotate, as rustc uses): their keys are the program's types, names and
//! numbers, not an adversary's, and the standard library's SipHash spent
//! a tenth of the time of writing IR hashing `Type`s. No map's order
//! decides what is written, so the object file is the same with either.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// The hasher's state.
#[derive(Default, Clone, Copy)]
pub(crate) struct Fx(u64);

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl Fx {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            let mut w = [0u8; 8];
            w.copy_from_slice(c);
            self.add(u64::from_le_bytes(w));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            let mut w = [0u8; 8];
            w[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(w));
        }
    }

    fn write_u8(&mut self, n: u8) {
        self.add(u64::from(n));
    }

    fn write_u16(&mut self, n: u16) {
        self.add(u64::from(n));
    }

    fn write_u32(&mut self, n: u32) {
        self.add(u64::from(n));
    }

    fn write_u64(&mut self, n: u64) {
        self.add(n);
    }

    fn write_usize(&mut self, n: usize) {
        self.add(n as u64);
    }
}

/// A map with [`Fx`] hashing.
pub(crate) type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<Fx>>;
