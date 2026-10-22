use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::MemoryStatistics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticsError {
    EmptySamples,
    TooManySamples,
    ArithmeticOverflow,
}

impl Display for StatisticsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySamples => formatter.write_str("memory samples cannot be empty"),
            Self::TooManySamples => formatter.write_str("memory sample count exceeds u32"),
            Self::ArithmeticOverflow => {
                formatter.write_str("memory statistics arithmetic overflowed")
            }
        }
    }
}

impl Error for StatisticsError {}

/// Summarizes byte samples using integer mean/median and nearest-rank percentiles.
///
/// # Errors
///
/// Returns an error for empty input, a sample count that cannot be represented by the
/// schema, or an arithmetic result that cannot be represented by the output fields.
pub fn summarize_bytes(samples: &[u64]) -> Result<MemoryStatistics, StatisticsError> {
    if samples.is_empty() {
        return Err(StatisticsError::EmptySamples);
    }
    let count = u32::try_from(samples.len()).map_err(|_| StatisticsError::TooManySamples)?;
    let mut ordered = samples.to_vec();
    ordered.sort_unstable();

    let sum = ordered.iter().map(|&value| u128::from(value)).sum::<u128>();
    let divisor = u128::from(count);
    let mean = u64::try_from(sum / divisor).map_err(|_| StatisticsError::ArithmeticOverflow)?;
    let median = if ordered.len() % 2 == 0 {
        let upper = ordered[ordered.len() / 2];
        let lower = ordered[ordered.len() / 2 - 1];
        lower + (upper - lower) / 2
    } else {
        ordered[ordered.len() / 2]
    };

    Ok(MemoryStatistics {
        count,
        min: ordered[0],
        max: ordered[ordered.len() - 1],
        mean,
        median,
        p50: nearest_rank(&ordered, count, 50)?,
        p95: nearest_rank(&ordered, count, 95)?,
    })
}

fn nearest_rank(ordered: &[u64], count: u32, percentile: u64) -> Result<u64, StatisticsError> {
    let rank = (u64::from(count) * percentile).div_ceil(100);
    let index =
        usize::try_from(rank.saturating_sub(1)).map_err(|_| StatisticsError::ArithmeticOverflow)?;
    ordered
        .get(index)
        .copied()
        .ok_or(StatisticsError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::{StatisticsError, summarize_bytes};

    #[test]
    fn nearest_rank_percentiles_are_stable_at_small_sample_boundaries() {
        let statistics = summarize_bytes(&[10, 20, 30, 40, 50]).unwrap();

        assert_eq!(statistics.count, 5);
        assert_eq!(statistics.min, 10);
        assert_eq!(statistics.max, 50);
        assert_eq!(statistics.mean, 30);
        assert_eq!(statistics.median, 30);
        assert_eq!(statistics.p50, 30);
        assert_eq!(statistics.p95, 50);
    }

    #[test]
    fn even_median_uses_a_checked_integer_midpoint() {
        let statistics = summarize_bytes(&[10, 20, 30, 40]).unwrap();

        assert_eq!(statistics.median, 25);
        assert_eq!(statistics.p50, 20);
        assert_eq!(statistics.p95, 40);
    }

    #[test]
    fn duplicate_and_singleton_samples_remain_exact() {
        let duplicates = summarize_bytes(&[7, 7, 7, 7]).unwrap();
        assert_eq!(duplicates.min, 7);
        assert_eq!(duplicates.max, 7);
        assert_eq!(duplicates.mean, 7);
        assert_eq!(duplicates.median, 7);
        assert_eq!(duplicates.p50, 7);
        assert_eq!(duplicates.p95, 7);

        let singleton = summarize_bytes(&[u64::MAX]).unwrap();
        assert_eq!(singleton.mean, u64::MAX);
        assert_eq!(singleton.median, u64::MAX);
    }

    #[test]
    fn mean_uses_a_wide_sum_without_overflow() {
        let statistics = summarize_bytes(&[u64::MAX, u64::MAX]).unwrap();

        assert_eq!(statistics.mean, u64::MAX);
        assert_eq!(statistics.median, u64::MAX);
    }

    #[test]
    fn empty_samples_are_an_explicit_error() {
        assert_eq!(summarize_bytes(&[]), Err(StatisticsError::EmptySamples));
    }
}
