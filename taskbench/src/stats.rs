//! Order statistics over latency samples.
//!
//! Percentiles use the nearest-rank definition (the smallest sample such that at
//! least p% of samples are less than or equal to it). The Go and Java controls
//! use the same definition so that the numbers are comparable.

/// Summary of a set of samples (nanoseconds unless stated otherwise).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Summary {
    pub n: usize,
    pub min: u64,
    pub p50: u64,
    pub p90: u64,
    pub p99: u64,
    pub p999: u64,
    pub max: u64,
    pub mean: f64,
}

/// Nearest-rank percentile of sorted samples; `p` in (0, 100].
pub fn percentile_sorted(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// Sorts `samples` in place and summarises them.
pub fn summarize(samples: &mut [u64]) -> Summary {
    if samples.is_empty() {
        return Summary::default();
    }
    samples.sort_unstable();
    let sum: u128 = samples.iter().map(|&v| v as u128).sum();
    Summary {
        n: samples.len(),
        min: samples[0],
        p50: percentile_sorted(samples, 50.0),
        p90: percentile_sorted(samples, 90.0),
        p99: percentile_sorted(samples, 99.0),
        p999: percentile_sorted(samples, 99.9),
        max: samples[samples.len() - 1],
        mean: sum as f64 / samples.len() as f64,
    }
}

/// Median of a small set of floating-point measurements (used for repeated runs).
pub fn median_f64(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank() {
        let mut v: Vec<u64> = (1..=100).collect();
        let s = summarize(&mut v);
        assert_eq!(s.p50, 50);
        assert_eq!(s.p99, 99);
        assert_eq!(s.p999, 100);
        assert_eq!(s.min, 1);
        assert_eq!(s.max, 100);
        assert!((s.mean - 50.5).abs() < 1e-9);
    }

    #[test]
    fn tiny_sets() {
        assert_eq!(summarize(&mut []).n, 0);
        let mut one = [7u64];
        let s = summarize(&mut one);
        assert_eq!((s.p50, s.p99, s.max), (7, 7, 7));
        assert_eq!(median_f64(&mut [3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median_f64(&mut [4.0, 1.0, 2.0, 3.0]), 2.5);
    }
}
