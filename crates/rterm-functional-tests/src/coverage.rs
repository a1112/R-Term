use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

use serde::{Deserialize, Serialize};

use crate::evidence::EvidencePayloadV1;
use crate::{BehaviorId, EvidenceEventV1, FunctionalSuite, ScenarioOutcome};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorEvidenceMapV1 {
    pub schema: u16,
    pub evidence: Vec<BehaviorEvidenceV1>,
}

impl BehaviorEvidenceMapV1 {
    /// Parses the executable-evidence mapping.
    ///
    /// # Errors
    ///
    /// Returns the TOML decoder error when the mapping is malformed.
    pub fn from_toml(contents: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(contents)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorEvidenceV1 {
    pub behavior_id: BehaviorId,
    pub source: ExecutableEvidenceSource,
    pub identity: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableEvidenceSource {
    Libtest,
    Playwright,
}

pub struct CoverageInputs<ScenarioReader, LibtestReader, PlaywrightReader> {
    pub scenario_ndjson: Vec<ScenarioReader>,
    pub libtest_listings: Vec<LibtestReader>,
    pub playwright_reports: Vec<PlaywrightReader>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BehaviorCoverageReportV1 {
    pub schema: u16,
    pub behaviors_total: usize,
    pub behaviors_covered: usize,
    pub subsystems_with_e2e: Vec<String>,
}

/// Verifies that actual scenario, libtest, and Playwright results cover the catalog.
///
/// # Errors
///
/// Returns all missing, orphaned, failed, or declarations-only evidence findings.
pub fn verify_behavior_coverage<ScenarioReader, LibtestReader, PlaywrightReader>(
    suite: &FunctionalSuite,
    map: &BehaviorEvidenceMapV1,
    inputs: CoverageInputs<ScenarioReader, LibtestReader, PlaywrightReader>,
) -> Result<BehaviorCoverageReportV1, Vec<String>>
where
    ScenarioReader: Read,
    LibtestReader: Read,
    PlaywrightReader: Read,
{
    let mut errors = Vec::new();
    if map.schema != 1 {
        errors.push(format!(
            "unsupported behavior evidence map schema {}",
            map.schema
        ));
    }
    let known: BTreeMap<_, _> = suite
        .catalog
        .behaviors
        .iter()
        .map(|behavior| (behavior.id.clone(), behavior))
        .collect();
    let mut covered = BTreeSet::new();
    let mut e2e_subsystems = BTreeSet::new();
    collect_scenario_evidence(
        suite,
        &known,
        inputs.scenario_ndjson,
        &mut covered,
        &mut e2e_subsystems,
        &mut errors,
    );
    let libtests = collect_libtests(inputs.libtest_listings, &mut errors);
    let playwright = collect_playwright(inputs.playwright_reports, &mut errors);
    collect_mapped_evidence(
        map,
        &known,
        &libtests,
        &playwright,
        &mut covered,
        &mut e2e_subsystems,
        &mut errors,
    );
    for behavior in known.values() {
        if !covered.contains(&behavior.id) {
            errors.push(format!(
                "behavior {} has no executable evidence",
                behavior.id.0
            ));
        }
        if !e2e_subsystems.contains(&behavior.subsystem) {
            errors.push(format!(
                "subsystem {} has no real-entry E2E journey",
                behavior.subsystem
            ));
        }
    }
    if errors.is_empty() {
        Ok(BehaviorCoverageReportV1 {
            schema: 1,
            behaviors_total: known.len(),
            behaviors_covered: covered.len(),
            subsystems_with_e2e: e2e_subsystems.into_iter().collect(),
        })
    } else {
        errors.sort();
        errors.dedup();
        Err(errors)
    }
}

fn collect_scenario_evidence(
    suite: &FunctionalSuite,
    known: &BTreeMap<BehaviorId, &crate::catalog::BehaviorV1>,
    readers: impl IntoIterator<Item = impl Read>,
    covered: &mut BTreeSet<BehaviorId>,
    e2e_subsystems: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    for reader in readers {
        match EvidenceEventV1::read_ndjson(reader) {
            Ok(events) => {
                let scenario_id = events
                    .first()
                    .map(|event| event.run_id.scenario_id.as_str());
                let passed = events.iter().any(|event| {
                    matches!(
                        event.payload,
                        EvidencePayloadV1::ScenarioFinished {
                            outcome: ScenarioOutcome::Passed
                        }
                    )
                });
                if let Some(scenario_id) = scenario_id {
                    if let Some(scenario) = suite.scenario(scenario_id) {
                        if passed {
                            let declared: BTreeSet<_> =
                                scenario.behavior_ids.iter().cloned().collect();
                            let observed: BTreeSet<_> = events
                                .iter()
                                .filter_map(|event| match &event.payload {
                                    EvidencePayloadV1::BehaviorObserved { behavior_id, .. } => {
                                        Some(BehaviorId(behavior_id.clone()))
                                    }
                                    _ => None,
                                })
                                .collect();
                            for behavior_id in observed.intersection(&declared) {
                                covered.insert(behavior_id.clone());
                                if let Some(behavior) = known.get(behavior_id) {
                                    e2e_subsystems.insert(behavior.subsystem.clone());
                                }
                            }
                            for behavior_id in declared.difference(&observed) {
                                errors.push(format!(
                                    "scenario {scenario_id:?} declared {} without a runtime observation",
                                    behavior_id.0
                                ));
                            }
                            for behavior_id in observed.difference(&declared) {
                                errors.push(format!(
                                    "scenario {scenario_id:?} observed undeclared behavior {}",
                                    behavior_id.0
                                ));
                            }
                        } else {
                            errors.push(format!("scenario {scenario_id:?} did not pass"));
                        }
                    } else {
                        errors.push(format!(
                            "evidence references unknown scenario {scenario_id:?}"
                        ));
                    }
                }
            }
            Err(error) => errors.push(format!("parse scenario NDJSON: {error}")),
        }
    }
}

fn collect_mapped_evidence(
    map: &BehaviorEvidenceMapV1,
    known: &BTreeMap<BehaviorId, &crate::catalog::BehaviorV1>,
    libtests: &BTreeSet<String>,
    playwright: &BTreeSet<String>,
    covered: &mut BTreeSet<BehaviorId>,
    e2e_subsystems: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let mut mapped = BTreeSet::new();
    for evidence in &map.evidence {
        if !known.contains_key(&evidence.behavior_id) {
            errors.push(format!(
                "evidence maps unknown behavior {}",
                evidence.behavior_id.0
            ));
        }
        if !mapped.insert((
            evidence.behavior_id.clone(),
            evidence.source,
            evidence.identity.clone(),
        )) {
            errors.push(format!(
                "duplicate evidence mapping for {}",
                evidence.identity
            ));
        }
        let executed = match evidence.source {
            ExecutableEvidenceSource::Libtest => libtests.contains(&evidence.identity),
            ExecutableEvidenceSource::Playwright => playwright.contains(&evidence.identity),
        };
        if executed {
            covered.insert(evidence.behavior_id.clone());
            if evidence.source == ExecutableEvidenceSource::Playwright
                && let Some(behavior) = known.get(&evidence.behavior_id)
            {
                e2e_subsystems.insert(behavior.subsystem.clone());
            }
        } else {
            errors.push(format!(
                "mapped evidence {} for {} was not executed successfully",
                evidence.identity, evidence.behavior_id.0
            ));
        }
    }
}

fn collect_libtests(
    readers: impl IntoIterator<Item = impl Read>,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut tests = BTreeSet::new();
    for mut reader in readers {
        let mut contents = String::new();
        if let Err(error) = reader.read_to_string(&mut contents) {
            errors.push(format!("read libtest listing: {error}"));
            continue;
        }
        for line in contents.lines() {
            let line = line.trim();
            if let Some(identity) = line
                .strip_prefix("test ")
                .and_then(|line| line.strip_suffix(" ... ok"))
            {
                tests.insert(identity.to_owned());
            }
        }
    }
    tests
}

fn collect_playwright(
    readers: impl IntoIterator<Item = impl Read>,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut tests = BTreeSet::new();
    for mut reader in readers {
        let mut contents = String::new();
        if let Err(error) = reader.read_to_string(&mut contents) {
            errors.push(format!("read Playwright report: {error}"));
            continue;
        }
        let report: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(report) => report,
            Err(error) => {
                errors.push(format!("parse Playwright report: {error}"));
                continue;
            }
        };
        collect_playwright_suites(&report, "", &mut tests, errors);
    }
    tests
}

fn collect_playwright_suites(
    value: &serde_json::Value,
    parent: &str,
    tests: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let Some(suites) = value.get("suites").and_then(serde_json::Value::as_array) else {
        return;
    };
    for suite in suites {
        let title = suite
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let prefix = if parent.is_empty() || title.is_empty() {
            format!("{parent}{title}")
        } else {
            format!("{parent} › {title}")
        };
        if let Some(specs) = suite.get("specs").and_then(serde_json::Value::as_array) {
            for spec in specs {
                let spec_title = spec
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let identity = if prefix.is_empty() {
                    spec_title.to_owned()
                } else {
                    format!("{prefix} › {spec_title}")
                };
                let results: Vec<_> = spec
                    .get("tests")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .flat_map(|test| {
                        test.get("results")
                            .and_then(serde_json::Value::as_array)
                            .into_iter()
                            .flatten()
                    })
                    .filter_map(|result| result.get("status").and_then(serde_json::Value::as_str))
                    .collect();
                if results == ["passed"] {
                    tests.insert(identity);
                } else if results.len() > 1 {
                    errors.push(format!(
                        "Playwright retry is forbidden for semantic evidence {identity:?}: {results:?}"
                    ));
                } else {
                    errors.push(format!("Playwright evidence {identity:?} did not pass"));
                }
            }
        }
        collect_playwright_suites(suite, &prefix, tests, errors);
    }
}
