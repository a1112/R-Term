use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{BehaviorId, ScenarioV1, Surface};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorCatalogV1 {
    pub schema: u16,
    pub behaviors: Vec<BehaviorV1>,
}

impl BehaviorCatalogV1 {
    /// Parses a versioned behavior catalog.
    ///
    /// # Errors
    ///
    /// Returns the TOML decoder error when the document is malformed.
    pub fn from_toml(contents: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(contents)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorV1 {
    pub id: BehaviorId,
    pub subsystem: String,
    pub summary: String,
    pub surfaces: Vec<Surface>,
}

/// Validates catalog identities and scenario references.
///
/// # Errors
///
/// Returns every catalog or reference violation found in the input.
pub fn validate_catalog(
    catalog: &BehaviorCatalogV1,
    scenarios: &[ScenarioV1],
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if catalog.schema != 1 {
        errors.push(format!(
            "unsupported behavior catalog schema {}",
            catalog.schema
        ));
    }
    let known = collect_known_behaviors(catalog, &mut errors);
    for scenario in scenarios {
        for behavior_id in &scenario.behavior_ids {
            match known.get(behavior_id) {
                None => errors.push(format!(
                    "scenario `{}` references unknown behavior `{}`",
                    scenario.id, behavior_id.0
                )),
                Some(behavior) if !behavior.surfaces.contains(&scenario.surface) => {
                    errors.push(format!(
                        "scenario `{}` surface `{}` is not declared by behavior `{}`",
                        scenario.id,
                        surface_name(scenario.surface),
                        behavior_id.0
                    ));
                }
                Some(_) => {}
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

const fn surface_name(surface: Surface) -> &'static str {
    match surface {
        Surface::Console => "console",
        Surface::HostTerminal => "host_terminal",
        Surface::NativeWindow => "native_window",
        Surface::Web => "web",
        Surface::Tauri => "tauri",
        Surface::Package => "package",
    }
}

fn collect_known_behaviors<'a>(
    catalog: &'a BehaviorCatalogV1,
    errors: &mut Vec<String>,
) -> BTreeMap<BehaviorId, &'a BehaviorV1> {
    let mut known = BTreeMap::new();
    for behavior in &catalog.behaviors {
        if known.insert(behavior.id.clone(), behavior).is_some() {
            errors.push(format!("duplicate behavior `{}`", behavior.id.0));
        }
    }
    known
}
