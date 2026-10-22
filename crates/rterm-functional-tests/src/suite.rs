use std::{
    collections::BTreeSet,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use crate::{BehaviorCatalogV1, ScenarioV1, validate_catalog};

#[derive(Clone, Debug)]
pub struct FunctionalSuite {
    pub root: PathBuf,
    pub catalog: BehaviorCatalogV1,
    pub scenarios: Vec<ScenarioV1>,
}

impl FunctionalSuite {
    /// Loads and validates a functional-test suite rooted at `root`.
    ///
    /// # Errors
    ///
    /// Returns an error when suite files cannot be read, parsed, or validated.
    pub fn load(root: impl AsRef<Path>) -> Result<Self, SuiteLoadError> {
        let root = root.as_ref().to_path_buf();
        let catalog_path = root.join("behaviors.toml");
        let catalog_contents =
            fs::read_to_string(&catalog_path).map_err(|source| SuiteLoadError::Read {
                path: catalog_path.clone(),
                source,
            })?;
        let catalog = BehaviorCatalogV1::from_toml(&catalog_contents).map_err(|source| {
            SuiteLoadError::ParseCatalog {
                path: catalog_path,
                source: Box::new(source),
            }
        })?;
        let scenarios_path = root.join("scenarios");
        let mut paths = Vec::new();
        for entry in fs::read_dir(&scenarios_path).map_err(|source| SuiteLoadError::Read {
            path: scenarios_path.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| SuiteLoadError::Read {
                path: scenarios_path.clone(),
                source,
            })?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) == Some("toml") {
                paths.push(path);
            }
        }
        paths.sort();
        let mut scenarios = Vec::with_capacity(paths.len());
        let mut ids = BTreeSet::new();
        for path in paths {
            let contents = fs::read_to_string(&path).map_err(|source| SuiteLoadError::Read {
                path: path.clone(),
                source,
            })?;
            let scenario = ScenarioV1::from_toml(&contents).map_err(|source| {
                SuiteLoadError::ParseScenario {
                    path: path.clone(),
                    source: Box::new(source),
                }
            })?;
            if !ids.insert(scenario.id.clone()) {
                return Err(SuiteLoadError::DuplicateScenario(scenario.id));
            }
            scenarios.push(scenario);
        }
        scenarios.sort_by(|left, right| left.id.cmp(&right.id));
        validate_catalog(&catalog, &scenarios).map_err(SuiteLoadError::InvalidCatalog)?;
        Ok(Self {
            root,
            catalog,
            scenarios,
        })
    }

    #[must_use]
    pub fn scenario(&self, id: &str) -> Option<&ScenarioV1> {
        self.scenarios
            .binary_search_by(|scenario| scenario.id.as_str().cmp(id))
            .ok()
            .map(|index| &self.scenarios[index])
    }
}

#[derive(Debug)]
pub enum SuiteLoadError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    ParseCatalog {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
    ParseScenario {
        path: PathBuf,
        source: Box<crate::scenario::ScenarioParseError>,
    },
    DuplicateScenario(String),
    InvalidCatalog(Vec<String>),
}

impl fmt::Display for SuiteLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(formatter, "read `{}`: {source}", path.display()),
            Self::ParseCatalog { path, source } => {
                write!(
                    formatter,
                    "parse behavior catalog `{}`: {source}",
                    path.display()
                )
            }
            Self::ParseScenario { path, source } => {
                write!(formatter, "parse scenario `{}`: {source}", path.display())
            }
            Self::DuplicateScenario(id) => write!(formatter, "duplicate scenario id `{id}`"),
            Self::InvalidCatalog(errors) => {
                write!(
                    formatter,
                    "invalid behavior evidence catalog: {}",
                    errors.join("; ")
                )
            }
        }
    }
}

impl Error for SuiteLoadError {}
