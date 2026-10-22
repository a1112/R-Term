use std::collections::BTreeSet;
use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ShardAssignment {
    pub index: usize,
    pub estimated_cost_ms: u64,
    pub scenario_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShardAssignmentError {
    ZeroShards,
    DuplicateScenario,
}

impl fmt::Display for ShardAssignmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroShards => formatter.write_str("shard count must be non-zero"),
            Self::DuplicateScenario => formatter.write_str("scenario IDs must be unique"),
        }
    }
}

impl Error for ShardAssignmentError {}

/// Assigns scenarios deterministically with longest-processing-time-first balancing.
///
/// # Errors
///
/// Returns an error for a zero shard count or duplicate scenario identity.
pub fn assign_lpt_shards<'a>(
    costs: impl IntoIterator<Item = (&'a str, u64)>,
    shard_count: usize,
) -> Result<Vec<ShardAssignment>, ShardAssignmentError> {
    if shard_count == 0 {
        return Err(ShardAssignmentError::ZeroShards);
    }
    let mut costs: Vec<_> = costs
        .into_iter()
        .map(|(id, cost)| (id.to_owned(), cost))
        .collect();
    let unique: BTreeSet<_> = costs.iter().map(|(id, _)| id.as_str()).collect();
    if unique.len() != costs.len() {
        return Err(ShardAssignmentError::DuplicateScenario);
    }
    costs.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let mut shards: Vec<_> = (0..shard_count)
        .map(|index| ShardAssignment {
            index,
            estimated_cost_ms: 0,
            scenario_ids: Vec::new(),
        })
        .collect();
    for (scenario_id, cost) in costs {
        let Some(shard) = shards
            .iter_mut()
            .min_by_key(|shard| (shard.estimated_cost_ms, shard.index))
        else {
            return Err(ShardAssignmentError::ZeroShards);
        };
        shard.estimated_cost_ms = shard.estimated_cost_ms.saturating_add(cost);
        shard.scenario_ids.push(scenario_id);
    }
    Ok(shards)
}
