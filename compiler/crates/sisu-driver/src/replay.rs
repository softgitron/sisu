//! Replay tokens (17, "Simulation and replay"): every failing test prints
//! one, and `sisu test --replay TOKEN FILE` runs that test again the same
//! way. Inside a test every source of nondeterminism is a capability fake
//! or a seed, so the token holds what reproduces the failure on any
//! machine: the content hash of the code, the toolchain's version, which
//! test, how the scheduler chose, the seed of the property's random cases
//! and, for a property test, the choices of the shrunk case.
//!
//! A token is one word of fields joined by `-`:
//!
//! ```text
//! 1-3f2a9c0d1e2b4a59-0.0.1-t2-s0-r0-c3.0.1
//! ```
//!
//! the token format's version, the content hash (FNV-1a of the file's
//! text, as the capability log writes it), the toolchain, then fields
//! named by a letter: `t` the test's index in declaration order, `s` the
//! scheduler's seed, `r` the case seed, `c` the case's choices separated
//! by `.` (absent for a test that is not a property), `p` the depth of a
//! PCT scheduler, `x` the choices of a bounded exhaustive search's
//! schedule, and `m` a run in the `sim` profile (`--sim`), with the
//! fault rate in parts per million.

use sisu_interp::Strategy;

/// The toolchain's version, as tokens and logs name it.
pub const TOOLCHAIN: &str = env!("CARGO_PKG_VERSION");

/// What a replay token holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub hash: u64,
    pub toolchain: String,
    pub test: usize,
    pub seed: u64,
    pub case_seed: u64,
    pub choices: Option<Vec<i64>>,
    pub strategy: Strategy,
    /// The `sim` profile's fault rate, in parts per million (`--sim`).
    pub sim: Option<u32>,
}

fn dotted<T: ToString>(xs: &[T]) -> String {
    xs.iter().map(T::to_string).collect::<Vec<_>>().join(".")
}

fn undotted<T: std::str::FromStr>(s: &str) -> Option<Vec<T>> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    s.split('.').map(|x| x.parse().ok()).collect()
}

impl Token {
    pub fn write(&self) -> String {
        let mut s = format!(
            "1-{:016x}-{}-t{}-s{}-r{}",
            self.hash, self.toolchain, self.test, self.seed, self.case_seed
        );
        if let Some(c) = &self.choices {
            s.push_str(&format!("-c{}", dotted(c)));
        }
        match &self.strategy {
            Strategy::Random => {}
            Strategy::Pct { depth } => s.push_str(&format!("-p{depth}")),
            Strategy::Exhaustive { prefix } => s.push_str(&format!("-x{}", dotted(prefix))),
        }
        if let Some(rate) = self.sim {
            s.push_str(&format!("-m{rate}"));
        }
        s
    }

    pub fn parse(text: &str) -> Option<Token> {
        let mut parts: Vec<&str> = text.split('-').collect();
        if parts.len() < 6 || parts[0] != "1" {
            return None;
        }
        let (mut test, mut seed, mut case_seed) = (None, None, None);
        let mut choices = None;
        let mut strategy = Strategy::Random;
        let mut sim = None;
        // From the end: the named fields follow the toolchain, whose
        // version may hold a `-` itself; `t` is the first of them.
        while parts.len() > 3 && test.is_none() {
            let last = parts[parts.len() - 1];
            let letter = last.chars().next()?;
            let rest = &last[1..];
            match letter {
                't' => test = Some(rest.parse().ok()?),
                's' => seed = Some(rest.parse().ok()?),
                'r' => case_seed = Some(rest.parse().ok()?),
                'c' => choices = Some(undotted(rest)?),
                'p' => strategy = Strategy::Pct { depth: rest.parse().ok()? },
                'x' => strategy = Strategy::Exhaustive { prefix: undotted(rest)? },
                'm' => sim = Some(rest.parse().ok()?),
                _ => return None,
            }
            parts.pop();
        }
        let hash = u64::from_str_radix(parts.get(1)?, 16).ok()?;
        let toolchain = parts.get(2..)?.join("-");
        if toolchain.is_empty() {
            return None;
        }
        Some(Token {
            hash,
            toolchain,
            test: test?,
            seed: seed?,
            case_seed: case_seed?,
            choices,
            strategy,
            sim,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_reads_back() {
        let t = Token {
            hash: 0x3f2a_9c0d_1e2b_4a59,
            toolchain: "0.0.1".into(),
            test: 2,
            seed: 7,
            case_seed: 0,
            choices: Some(vec![3, 0, 1]),
            strategy: Strategy::Random,
            sim: None,
        };
        assert_eq!(t.write(), "1-3f2a9c0d1e2b4a59-0.0.1-t2-s7-r0-c3.0.1");
        assert_eq!(Token::parse(&t.write()), Some(t.clone()));
        let plain = Token { choices: None, ..t.clone() };
        assert_eq!(Token::parse(&plain.write()), Some(plain));
        let pct = Token { strategy: Strategy::Pct { depth: 3 }, sim: Some(20000), ..t.clone() };
        assert_eq!(pct.write(), "1-3f2a9c0d1e2b4a59-0.0.1-t2-s7-r0-c3.0.1-p3-m20000");
        assert_eq!(Token::parse(&pct.write()), Some(pct));
        let x = Token {
            strategy: Strategy::Exhaustive { prefix: vec![0, 2, 1] },
            toolchain: "0.1.0-rc1".into(),
            ..t
        };
        assert_eq!(Token::parse(&x.write()), Some(x));
        assert_eq!(Token::parse("not a token"), None);
        assert_eq!(Token::parse("1-zz-0.0.1-t0-s0-r0"), None);
    }
}
