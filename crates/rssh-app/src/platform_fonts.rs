use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs, io,
    path::PathBuf,
};

use rssh_fonts::{FontCatalog, FontSource};
use rterm_render_core::terminal_bytes_content_digest;

#[cfg(test)]
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const PLATFORM_FONT_POLICY_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct FontKey(u16);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum FontCoverage {
    Primary,
    Cjk,
    Arabic,
    Devanagari,
    Hebrew,
    Symbols,
    Emoji,
}

impl FontCoverage {
    const fn tag(self) -> u8 {
        match self {
            Self::Primary => 0,
            Self::Cjk => 1,
            Self::Arabic => 2,
            Self::Devanagari => 3,
            Self::Hebrew => 4,
            Self::Symbols => 5,
            Self::Emoji => 6,
        }
    }
}

#[derive(Clone)]
enum IndexedFontLocator {
    File(PathBuf),
    Embedded(&'static [u8]),
}

#[derive(Clone)]
pub(crate) struct IndexedFont {
    key: FontKey,
    label: &'static str,
    coverage: FontCoverage,
    emergency: bool,
    locator: IndexedFontLocator,
    #[cfg(test)]
    materializations: Arc<AtomicUsize>,
    #[cfg(test)]
    availability_probes: Arc<AtomicUsize>,
}

impl IndexedFont {
    fn file(key: FontKey, label: &'static str, coverage: FontCoverage, path: PathBuf) -> Self {
        Self {
            key,
            label,
            coverage,
            emergency: false,
            locator: IndexedFontLocator::File(path),
            #[cfg(test)]
            materializations: Arc::new(AtomicUsize::new(0)),
            #[cfg(test)]
            availability_probes: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[cfg(test)]
    fn embedded(
        key: FontKey,
        label: &'static str,
        coverage: FontCoverage,
        bytes: &'static [u8],
    ) -> Self {
        Self {
            key,
            label,
            coverage,
            emergency: true,
            locator: IndexedFontLocator::Embedded(bytes),
            materializations: Arc::new(AtomicUsize::new(0)),
            availability_probes: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn embedded_loader(
        key: FontKey,
        label: &'static str,
        coverage: FontCoverage,
        emergency: bool,
        bytes: &'static [u8],
    ) -> Self {
        Self {
            key,
            label,
            coverage,
            emergency,
            locator: IndexedFontLocator::Embedded(bytes),
            #[cfg(test)]
            materializations: Arc::new(AtomicUsize::new(0)),
            #[cfg(test)]
            availability_probes: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn is_available(&self) -> bool {
        #[cfg(test)]
        self.availability_probes.fetch_add(1, Ordering::Relaxed);
        match &self.locator {
            IndexedFontLocator::File(path) => path.is_file(),
            IndexedFontLocator::Embedded(_) => true,
        }
    }

    fn materialize(&self) -> Result<FontSource, Box<dyn Error>> {
        #[cfg(test)]
        self.materializations.fetch_add(1, Ordering::Relaxed);
        let bytes = match &self.locator {
            IndexedFontLocator::File(path) => fs::read(path)?,
            IndexedFontLocator::Embedded(bytes) => bytes.to_vec(),
        };
        Ok(FontSource::new(self.label, bytes))
    }

    #[cfg(all(test, not(feature = "rterm-legacy-0-1")))]
    const fn retains_no_font_bytes(&self) -> bool {
        matches!(
            &self.locator,
            IndexedFontLocator::File(_) | IndexedFontLocator::Embedded(_)
        )
    }

    #[cfg(all(test, not(feature = "rterm-legacy-0-1")))]
    fn materialization_count(&self) -> usize {
        self.materializations.load(Ordering::Relaxed)
    }

    #[cfg(all(test, not(feature = "rterm-legacy-0-1")))]
    fn availability_probe_count(&self) -> usize {
        self.availability_probes.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "rterm-legacy-0-1",
    allow(
        dead_code,
        reason = "retain explicit unsupported modes for the legacy capability boundary"
    )
)]
pub(crate) enum FontCatalogMode {
    #[cfg(feature = "diagnostic-tools")]
    CurrentCopied,
    #[cfg(feature = "diagnostic-tools")]
    SharedAll,
    Lazy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameFontPlan {
    required: Vec<FontKey>,
    catalog_fingerprint: [u8; 32],
    unresolved: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CatalogActivation {
    Unchanged,
    StableMissingGlyph,
    CatalogExpanded {
        previous_generation: u64,
        catalog_generation: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    dead_code,
    reason = "the safe resource summary is consumed by the next diagnostic wiring task"
)]
pub(crate) struct PlatformFontDiagnostics {
    pub(crate) policy_version: u32,
    pub(crate) indexed_source_count: usize,
    pub(crate) active_source_count: usize,
    pub(crate) initial_catalog_source_count: usize,
    pub(crate) retained_source_bytes: usize,
    pub(crate) catalog_builds: u64,
    pub(crate) generation: u64,
    pub(crate) index_fingerprint: [u8; 32],
    pub(crate) catalog_fingerprint: [u8; 32],
    pub(crate) inventory_fingerprint: [u8; 32],
}

pub(crate) struct PlatformFontRepository {
    policy_version: u32,
    indexed: Vec<IndexedFont>,
    active: BTreeMap<FontKey, FontSource>,
    activation_order: Vec<FontKey>,
    initial_catalog_source_count: usize,
    #[allow(
        dead_code,
        reason = "the index digest is consumed by the next diagnostic wiring task"
    )]
    index_fingerprint: [u8; 32],
    catalog_fingerprint: [u8; 32],
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    catalog_builds: u64,
    generation: u64,
}

impl PlatformFontRepository {
    fn new(policy_version: u32, indexed: Vec<IndexedFont>) -> Self {
        let index_fingerprint = indexed_fingerprint(policy_version, &indexed);
        let catalog_fingerprint = active_fingerprint(policy_version, &[], &BTreeMap::new());
        Self {
            policy_version,
            indexed,
            active: BTreeMap::new(),
            activation_order: Vec::new(),
            initial_catalog_source_count: 0,
            index_fingerprint,
            catalog_fingerprint,
            #[cfg(not(feature = "rterm-legacy-0-1"))]
            catalog_builds: 0,
            generation: 0,
        }
    }

    pub(crate) fn production_index() -> Self {
        Self::production_index_for_os(std::env::consts::OS)
    }

    pub(crate) fn production_index_for_os(os: &str) -> Self {
        let mut indexed = platform_candidates(os);
        indexed.extend(emergency_candidates());
        Self::new(PLATFORM_FONT_POLICY_VERSION, indexed)
    }

    #[cfg(test)]
    pub(crate) fn late_missing_fixture() -> Self {
        Self::new(
            7,
            vec![
                IndexedFont::embedded_loader(
                    FontKey(1),
                    "latin",
                    FontCoverage::Primary,
                    false,
                    emergency_latin_bytes(),
                ),
                IndexedFont::embedded_loader(
                    FontKey(2),
                    "false-cjk-latin-bytes",
                    FontCoverage::Cjk,
                    false,
                    emergency_latin_bytes(),
                ),
                IndexedFont::embedded_loader(
                    FontKey(3),
                    "real-cjk",
                    FontCoverage::Cjk,
                    false,
                    emergency_cjk_bytes(),
                ),
            ],
        )
    }

    #[cfg(test)]
    pub(crate) fn repeated_late_missing_fixture() -> Self {
        Self::new(
            7,
            vec![
                IndexedFont::embedded_loader(
                    FontKey(1),
                    "latin",
                    FontCoverage::Primary,
                    false,
                    emergency_latin_bytes(),
                ),
                IndexedFont::embedded_loader(
                    FontKey(2),
                    "false-cjk-one",
                    FontCoverage::Cjk,
                    false,
                    emergency_latin_bytes(),
                ),
                IndexedFont::embedded_loader(
                    FontKey(3),
                    "false-cjk-two",
                    FontCoverage::Cjk,
                    false,
                    emergency_latin_bytes(),
                ),
                IndexedFont::embedded_loader(
                    FontKey(4),
                    "deferred-real-cjk",
                    FontCoverage::Cjk,
                    false,
                    emergency_cjk_bytes(),
                ),
            ],
        )
    }

    #[cfg(test)]
    pub(crate) fn invalid_late_missing_fixture() -> Self {
        static INVALID_FONT: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7];
        Self::new(
            7,
            vec![
                IndexedFont::embedded_loader(
                    FontKey(1),
                    "latin",
                    FontCoverage::Primary,
                    false,
                    emergency_latin_bytes(),
                ),
                IndexedFont::embedded_loader(
                    FontKey(2),
                    "false-cjk",
                    FontCoverage::Cjk,
                    false,
                    emergency_latin_bytes(),
                ),
                IndexedFont::embedded_loader(
                    FontKey(3),
                    "invalid-cjk",
                    FontCoverage::Cjk,
                    false,
                    INVALID_FONT,
                ),
            ],
        )
    }

    #[cfg(all(test, not(feature = "rterm-legacy-0-1")))]
    fn indexed_file_count(&self) -> usize {
        self.indexed
            .iter()
            .filter(|source| matches!(source.locator, IndexedFontLocator::File(_)))
            .count()
    }

    pub(crate) fn build_catalog(
        &mut self,
        mode: FontCatalogMode,
    ) -> Result<FontCatalog, Box<dyn Error>> {
        match mode {
            #[cfg(all(feature = "diagnostic-tools", not(feature = "rterm-legacy-0-1")))]
            FontCatalogMode::CurrentCopied => self.build_current_copied(),
            #[cfg(all(feature = "diagnostic-tools", not(feature = "rterm-legacy-0-1")))]
            FontCatalogMode::SharedAll => self.build_all_once(),
            #[cfg(all(feature = "diagnostic-tools", feature = "rterm-legacy-0-1"))]
            FontCatalogMode::CurrentCopied | FontCatalogMode::SharedAll => {
                Err(crate::rterm_compat::unsupported_diagnostics())
            }
            FontCatalogMode::Lazy => self.build_lazy(),
        }
    }

    #[cfg(all(feature = "diagnostic-tools", not(feature = "rterm-legacy-0-1")))]
    fn build_current_copied(&mut self) -> Result<FontCatalog, Box<dyn Error>> {
        let mut emergency = Vec::new();
        let mut platform = Vec::new();
        for indexed in self.indexed.iter().filter(|source| source.is_available()) {
            if indexed.emergency {
                emergency.push((indexed.key, indexed.materialize()?));
            } else if let Ok(source) = indexed.materialize() {
                platform.push((indexed.key, source));
            }
        }
        let mut catalog = FontCatalog::from_sources_copied_for_diagnostics(
            "en-US",
            emergency.iter().map(|(_, source)| source.clone()),
        )?;
        let initial_catalog_source_count = emergency.len();
        let mut committed = emergency;
        for (key, source) in platform {
            if catalog.load_source(source.clone()).is_ok() {
                committed.push((key, source));
            }
        }
        self.commit_initial_catalog(&catalog, committed, initial_catalog_source_count);
        Ok(catalog)
    }

    #[cfg(all(feature = "diagnostic-tools", not(feature = "rterm-legacy-0-1")))]
    fn build_all_once(&mut self) -> Result<FontCatalog, Box<dyn Error>> {
        let sources = self.materialize_all_in_catalog_order()?;
        let catalog = FontCatalog::from_sources_shared_for_diagnostics(
            "en-US",
            sources.iter().map(|(_, source)| source.clone()),
        )?;
        let initial_catalog_source_count = sources.len();
        self.commit_initial_catalog(&catalog, sources, initial_catalog_source_count);
        Ok(catalog)
    }

    fn build_lazy(&mut self) -> Result<FontCatalog, Box<dyn Error>> {
        let indexed = self
            .indexed
            .iter()
            .find(|source| source.coverage == FontCoverage::Primary && source.is_available())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no primary font source"))?;
        let sources = vec![(indexed.key, indexed.materialize()?)];
        let catalog =
            FontCatalog::from_sources("en-US", sources.iter().map(|(_, source)| source.clone()))?;
        let initial_catalog_source_count = sources.len();
        self.commit_initial_catalog(&catalog, sources, initial_catalog_source_count);
        Ok(catalog)
    }

    #[cfg(all(feature = "diagnostic-tools", not(feature = "rterm-legacy-0-1")))]
    fn materialize_all_in_catalog_order(
        &self,
    ) -> Result<Vec<(FontKey, FontSource)>, Box<dyn Error>> {
        let mut sources = Vec::new();
        for emergency in [true, false] {
            for indexed in self
                .indexed
                .iter()
                .filter(|source| source.emergency == emergency && source.is_available())
            {
                sources.push((indexed.key, indexed.materialize()?));
            }
        }
        Ok(sources)
    }

    fn commit_initial_catalog(
        &mut self,
        catalog: &FontCatalog,
        sources: Vec<(FontKey, FontSource)>,
        initial_catalog_source_count: usize,
    ) {
        self.active = sources.iter().cloned().collect();
        self.activation_order = sources.into_iter().map(|(key, _)| key).collect();
        self.initial_catalog_source_count = initial_catalog_source_count;
        self.catalog_fingerprint =
            active_fingerprint(self.policy_version, &self.activation_order, &self.active);
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        {
            self.catalog_builds = catalog.memory_metrics().catalog_builds;
        }
        self.generation = catalog.generation();
    }

    pub(crate) fn frame_plan(&self, text: &str) -> FrameFontPlan {
        self.plan_required_sources(text.chars(), true)
    }

    fn plan_required_sources(
        &self,
        characters: impl IntoIterator<Item = char>,
        skip_active_coverage: bool,
    ) -> FrameFontPlan {
        let mut required = Vec::new();
        let mut visited_coverages = BTreeSet::new();
        let mut unresolved = false;
        for character in characters {
            if is_ignorable_character(character) {
                continue;
            }
            let Some(coverage) = coverage_for_character(character) else {
                unresolved = true;
                continue;
            };
            if !visited_coverages.insert(coverage) {
                continue;
            }
            if skip_active_coverage && self.active_covers(coverage) {
                continue;
            }
            let candidate = self.indexed.iter().find(|source| {
                source.coverage == coverage
                    && source.is_available()
                    && !self.active.contains_key(&source.key)
            });
            match candidate {
                Some(candidate) => required.push(candidate.key),
                None => unresolved = true,
            }
        }
        FrameFontPlan {
            required,
            catalog_fingerprint: self.catalog_fingerprint,
            unresolved,
        }
    }

    fn active_covers(&self, coverage: FontCoverage) -> bool {
        self.activation_order.iter().any(|key| {
            self.indexed
                .iter()
                .find(|source| source.key == *key)
                .is_some_and(|source| source.coverage == coverage)
        })
    }

    pub(crate) fn activate_plan(
        &mut self,
        plan: &FrameFontPlan,
        catalog: &mut FontCatalog,
    ) -> Result<CatalogActivation, Box<dyn Error>> {
        if plan.catalog_fingerprint != self.catalog_fingerprint {
            return Err(io::Error::other("stale platform-font frame plan").into());
        }
        if plan.required.is_empty() {
            return Ok(if plan.unresolved {
                CatalogActivation::StableMissingGlyph
            } else {
                CatalogActivation::Unchanged
            });
        }
        let mut additions = Vec::with_capacity(plan.required.len());
        for key in &plan.required {
            if self.active.contains_key(key) {
                continue;
            }
            let indexed = self
                .indexed
                .iter()
                .find(|source| source.key == *key)
                .ok_or_else(|| io::Error::other("unknown platform-font key"))?;
            additions.push((*key, indexed.materialize()?));
        }
        if additions.is_empty() {
            return Ok(CatalogActivation::Unchanged);
        }
        let previous_generation = catalog.generation();
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        let catalog_generation =
            catalog.load_sources(additions.iter().map(|(_, source)| source.clone()))?;
        #[cfg(feature = "rterm-legacy-0-1")]
        let catalog_generation = {
            if previous_generation != self.generation {
                return Err(io::Error::other(
                    "legacy font catalog disagrees with repository epoch",
                )
                .into());
            }
            let ordered = self
                .activation_order
                .iter()
                .map(|key| {
                    self.active
                        .get(key)
                        .cloned()
                        .ok_or_else(|| io::Error::other("missing active legacy font source"))
                })
                .chain(additions.iter().map(|(_, source)| Ok(source.clone())))
                .collect::<Result<Vec<_>, _>>()?;
            crate::rterm_compat::replace_catalog(catalog, &ordered)?
        };
        for (key, source) in additions {
            self.active.insert(key, source);
            self.activation_order.push(key);
        }
        self.catalog_fingerprint =
            active_fingerprint(self.policy_version, &self.activation_order, &self.active);
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        {
            self.catalog_builds = catalog.memory_metrics().catalog_builds;
        }
        self.generation = catalog_generation;
        Ok(CatalogActivation::CatalogExpanded {
            previous_generation,
            catalog_generation,
        })
    }

    pub(crate) fn preflight_text(
        &mut self,
        text: &str,
        catalog: &mut FontCatalog,
    ) -> Result<CatalogActivation, Box<dyn Error>> {
        let plan = self.frame_plan(text);
        self.activate_plan(&plan, catalog)
    }

    pub(crate) fn activate_missing_glyphs(
        &mut self,
        missing_glyphs: &[char],
        catalog: &mut FontCatalog,
    ) -> Result<CatalogActivation, Box<dyn Error>> {
        let plan = self.plan_required_sources(missing_glyphs.iter().copied(), false);
        self.activate_plan(&plan, catalog)
    }

    pub(crate) fn has_pending_fallbacks(&self, missing_glyphs: &[char]) -> bool {
        !self
            .plan_required_sources(missing_glyphs.iter().copied(), false)
            .required
            .is_empty()
    }

    pub(crate) fn preflight_snapshot(
        &mut self,
        snapshot: &rterm_render_core::TerminalRenderSnapshot,
        catalog: &mut FontCatalog,
    ) -> Result<CatalogActivation, Box<dyn Error>> {
        let mut text = String::new();
        for cell in snapshot.iter_cells().filter(|cell| !cell.continuation) {
            text.push_str(&cell.text);
        }
        self.preflight_text(&text, catalog)
    }

    pub(crate) fn rebuild_catalog_from_active(
        &self,
        mode: FontCatalogMode,
    ) -> Result<FontCatalog, Box<dyn Error>> {
        let ordered = self
            .activation_order
            .iter()
            .filter_map(|key| self.active.get(key).cloned())
            .collect::<Vec<_>>();
        match mode {
            #[cfg(all(feature = "diagnostic-tools", not(feature = "rterm-legacy-0-1")))]
            FontCatalogMode::CurrentCopied => {
                let mut emergency = Vec::new();
                let mut platform = Vec::new();
                for key in &self.activation_order {
                    let Some(source) = self.active.get(key).cloned() else {
                        continue;
                    };
                    let is_emergency = self
                        .indexed
                        .iter()
                        .find(|indexed| indexed.key == *key)
                        .is_some_and(|indexed| indexed.emergency);
                    if is_emergency {
                        emergency.push(source);
                    } else {
                        platform.push(source);
                    }
                }
                let mut catalog =
                    FontCatalog::from_sources_copied_for_diagnostics("en-US", emergency)?;
                for source in platform {
                    catalog.load_source(source)?;
                }
                Ok(catalog)
            }
            #[cfg(all(feature = "diagnostic-tools", not(feature = "rterm-legacy-0-1")))]
            FontCatalogMode::SharedAll => self.rebuild_ordered_catalog_at_current_epoch(&ordered),
            #[cfg(all(feature = "diagnostic-tools", feature = "rterm-legacy-0-1"))]
            FontCatalogMode::CurrentCopied | FontCatalogMode::SharedAll => {
                Err(crate::rterm_compat::unsupported_diagnostics())
            }
            FontCatalogMode::Lazy => self.rebuild_ordered_catalog_at_current_epoch(&ordered),
        }
    }

    #[cfg(feature = "rterm-legacy-0-1")]
    fn rebuild_ordered_catalog_at_current_epoch(
        &self,
        ordered: &[FontSource],
    ) -> Result<FontCatalog, Box<dyn Error>> {
        crate::rterm_compat::rebuild_catalog(ordered, self.generation)
    }

    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn rebuild_ordered_catalog_at_current_epoch(
        &self,
        ordered: &[FontSource],
    ) -> Result<FontCatalog, Box<dyn Error>> {
        if self.generation == 0
            || self.generation != self.catalog_builds
            || usize::try_from(self.generation).map_or(true, |builds| builds > ordered.len())
        {
            return Err(io::Error::other(
                "platform-font repository epoch cannot be reconstructed from active sources",
            )
            .into());
        }
        let build_count = usize::try_from(self.generation)
            .map_err(|_| io::Error::other("platform-font generation exceeds usize"))?;
        let initial_count = ordered.len().saturating_sub(build_count.saturating_sub(1));
        let mut catalog = FontCatalog::from_sources("en-US", ordered[..initial_count].to_vec())?;
        for source in &ordered[initial_count..] {
            catalog.load_source(source.clone())?;
        }
        if catalog.generation() != self.generation
            || catalog.memory_metrics().catalog_builds != self.catalog_builds
        {
            return Err(io::Error::other(
                "recovered platform-font catalog epoch disagrees with repository diagnostics",
            )
            .into());
        }
        Ok(catalog)
    }

    #[allow(
        dead_code,
        reason = "the safe resource summary is consumed by the next diagnostic wiring task"
    )]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    #[allow(
        clippy::unnecessary_wraps,
        reason = "legacy profile returns an unsupported error through the same boundary"
    )]
    pub(crate) fn diagnostics(&self) -> Result<PlatformFontDiagnostics, Box<dyn Error>> {
        Ok(PlatformFontDiagnostics {
            policy_version: self.policy_version,
            indexed_source_count: self.indexed.len(),
            active_source_count: self.active.len(),
            initial_catalog_source_count: self.initial_catalog_source_count,
            retained_source_bytes: self.active.values().fold(0usize, |total, source| {
                total.saturating_add(source.bytes().len())
            }),
            catalog_builds: self.catalog_builds,
            generation: self.generation,
            index_fingerprint: self.index_fingerprint,
            catalog_fingerprint: self.catalog_fingerprint,
            inventory_fingerprint: font_inventory_fingerprint(
                self.active
                    .values()
                    .map(|source| (source.label.as_str(), source.bytes())),
            ),
        })
    }

    #[cfg(feature = "rterm-legacy-0-1")]
    #[allow(
        dead_code,
        clippy::unused_self,
        reason = "both profiles retain an explicit instance diagnostic boundary even when callers are compiled out"
    )]
    pub(crate) fn diagnostics(&self) -> Result<PlatformFontDiagnostics, Box<dyn Error>> {
        Err(crate::rterm_compat::unsupported_diagnostics())
    }

    #[cfg(all(test, not(feature = "rterm-legacy-0-1")))]
    fn active_labels(&self) -> Vec<&str> {
        self.activation_order
            .iter()
            .filter_map(|key| {
                self.indexed
                    .iter()
                    .find(|source| source.key == *key)
                    .map(|source| source.label)
            })
            .collect()
    }

    #[cfg(all(test, not(feature = "rterm-legacy-0-1")))]
    fn availability_probe_count(&self) -> usize {
        self.indexed
            .iter()
            .map(IndexedFont::availability_probe_count)
            .sum()
    }
}

pub(crate) const fn production_font_catalog_mode() -> FontCatalogMode {
    FontCatalogMode::Lazy
}

fn indexed_fingerprint(policy_version: u32, indexed: &[IndexedFont]) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"rssh-platform-font-index-v1\0");
    bytes.extend_from_slice(&policy_version.to_le_bytes());
    for source in indexed {
        bytes.extend_from_slice(&source.key.0.to_le_bytes());
        bytes.push(source.coverage.tag());
        bytes.push(u8::from(source.emergency));
        bytes.extend_from_slice(source.label.as_bytes());
        bytes.push(0);
        match &source.locator {
            IndexedFontLocator::File(path) => {
                bytes.push(0);
                bytes.extend_from_slice(path.as_os_str().to_string_lossy().as_bytes());
            }
            IndexedFontLocator::Embedded(_) => bytes.push(1),
        }
        bytes.push(0xff);
    }
    terminal_bytes_content_digest(&bytes)
}

fn active_fingerprint(
    policy_version: u32,
    activation_order: &[FontKey],
    active: &BTreeMap<FontKey, FontSource>,
) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"rssh-platform-font-active-v2\0");
    bytes.extend_from_slice(&policy_version.to_le_bytes());
    bytes.extend_from_slice(
        &u64::try_from(activation_order.len())
            .expect("platform font activation count fits u64")
            .to_le_bytes(),
    );
    for key in activation_order {
        let Some(source) = active.get(key) else {
            continue;
        };
        push_fingerprint_field(&mut bytes, &key.0.to_le_bytes());
        push_fingerprint_field(&mut bytes, source.label.as_bytes());
        push_fingerprint_field(&mut bytes, &terminal_bytes_content_digest(source.bytes()));
    }
    terminal_bytes_content_digest(&bytes)
}

#[cfg(not(feature = "rterm-legacy-0-1"))]
fn font_inventory_fingerprint<'a>(
    sources: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> [u8; 32] {
    let mut records = sources
        .into_iter()
        .map(|(label, bytes)| {
            let mut record = Vec::new();
            push_fingerprint_field(&mut record, label.as_bytes());
            push_fingerprint_field(&mut record, &terminal_bytes_content_digest(bytes));
            record
        })
        .collect::<Vec<_>>();
    records.sort_unstable();

    let mut envelope = Vec::new();
    envelope.extend_from_slice(b"rssh-platform-font-inventory-v1\0");
    envelope.extend_from_slice(
        &u64::try_from(records.len())
            .expect("platform font inventory count fits u64")
            .to_le_bytes(),
    );
    for record in records {
        push_fingerprint_field(&mut envelope, &record);
    }
    terminal_bytes_content_digest(&envelope)
}

fn push_fingerprint_field(envelope: &mut Vec<u8>, field: &[u8]) {
    envelope.extend_from_slice(
        &u64::try_from(field.len())
            .expect("platform font fingerprint field length fits u64")
            .to_le_bytes(),
    );
    envelope.extend_from_slice(field);
}

fn coverage_for_character(character: char) -> Option<FontCoverage> {
    let scalar = u32::from(character);
    if character.is_ascii() || matches!(scalar, 0x00a0..=0x024f) {
        return Some(FontCoverage::Primary);
    }
    if matches!(scalar, 0x0590..=0x05ff | 0xfb1d..=0xfb4f) {
        return Some(FontCoverage::Hebrew);
    }
    if matches!(scalar, 0x0600..=0x08ff | 0xfb50..=0xfdff | 0xfe70..=0xfeff) {
        return Some(FontCoverage::Arabic);
    }
    if matches!(scalar, 0x0900..=0x097f | 0xa8e0..=0xa8ff) {
        return Some(FontCoverage::Devanagari);
    }
    // Include compatibility/vertical/small forms and all Hangul Jamo blocks,
    // not just ideographs and precomposed syllables.
    if matches!(
        scalar,
        0x1100..=0x11ff | 0x2e80..=0x9fff | 0xa960..=0xa97f | 0xac00..=0xd7ff
            | 0xf900..=0xfaff | 0xfe10..=0xfe1f | 0xfe30..=0xfe6f
            | 0xff00..=0xffef | 0x20000..=0x3134f
    ) {
        return Some(FontCoverage::Cjk);
    }
    if matches!(scalar, 0x1f000..=0x1faff | 0x2600..=0x27bf) {
        return Some(FontCoverage::Emoji);
    }
    if matches!(scalar, 0x2000..=0x25ff | 0x2b00..=0x2bff) {
        return Some(FontCoverage::Symbols);
    }
    None
}

pub(crate) fn is_ignorable_character(character: char) -> bool {
    character.is_whitespace()
        || matches!(u32::from(character), 0x0300..=0x036f | 0x200b..=0x200f | 0xfe00..=0xfe0f | 0xe0100..=0xe01ef)
}

#[expect(
    clippy::too_many_lines,
    reason = "the reviewed cross-platform candidate table remains one auditable policy inventory"
)]
fn platform_candidates(os: &str) -> Vec<IndexedFont> {
    let candidates: &[(&str, &str, FontCoverage)] = match os {
        "windows" => &[
            (
                "CascadiaMono.system.ttf",
                r"C:\Windows\Fonts\CascadiaMono.ttf",
                FontCoverage::Primary,
            ),
            (
                "CascadiaCode.system.ttf",
                r"C:\Windows\Fonts\CascadiaCode.ttf",
                FontCoverage::Primary,
            ),
            (
                "SourceCodePro.system.ttf",
                r"C:\Windows\Fonts\SourceCodePro-Regular.ttf",
                FontCoverage::Primary,
            ),
            (
                "Consolas.system.ttf",
                r"C:\Windows\Fonts\consola.ttf",
                FontCoverage::Primary,
            ),
            (
                "NotoSansSC.system.ttf",
                r"C:\Windows\Fonts\NotoSansSC-VF.ttf",
                FontCoverage::Cjk,
            ),
            (
                "NotoSansJP.system.ttf",
                r"C:\Windows\Fonts\NotoSansJP-VF.ttf",
                FontCoverage::Cjk,
            ),
            (
                "MicrosoftYaHei.system.ttc",
                r"C:\Windows\Fonts\msyh.ttc",
                FontCoverage::Cjk,
            ),
            (
                "Meiryo.system.ttc",
                r"C:\Windows\Fonts\meiryo.ttc",
                FontCoverage::Cjk,
            ),
            (
                "MalgunGothic.system.ttf",
                r"C:\Windows\Fonts\malgun.ttf",
                FontCoverage::Cjk,
            ),
            (
                "SegoeUI.system.ttf",
                r"C:\Windows\Fonts\segoeui.ttf",
                FontCoverage::Hebrew,
            ),
            (
                "NirmalaUI.system.ttc",
                r"C:\Windows\Fonts\Nirmala.ttc",
                FontCoverage::Devanagari,
            ),
            (
                "SegoeUIEmoji.system.ttf",
                r"C:\Windows\Fonts\seguiemj.ttf",
                FontCoverage::Emoji,
            ),
        ],
        "linux" => &[
            (
                "NotoSansMono.system.ttf",
                "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
                FontCoverage::Primary,
            ),
            (
                "DejaVuSansMono.system.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
                FontCoverage::Primary,
            ),
            (
                "NotoSansCJK.system.ttc",
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                FontCoverage::Cjk,
            ),
            (
                "NotoSansArabic.system.ttf",
                "/usr/share/fonts/truetype/noto/NotoSansArabic-Regular.ttf",
                FontCoverage::Arabic,
            ),
            (
                "NotoSansDevanagari.system.ttf",
                "/usr/share/fonts/truetype/noto/NotoSansDevanagari-Regular.ttf",
                FontCoverage::Devanagari,
            ),
            (
                "NotoColorEmoji.system.ttf",
                "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
                FontCoverage::Emoji,
            ),
            // Arch (including ForgeOS) installs these packages outside the
            // Debian truetype/opentype directories. Keep both layouts in the
            // lazy inventory; unavailable paths are never materialized.
            (
                "NotoSansMono.arch.system.ttf",
                "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
                FontCoverage::Primary,
            ),
            (
                "DejaVuSansMono.arch.system.ttf",
                "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
                FontCoverage::Primary,
            ),
            (
                "NotoSans.arch.system.ttf",
                "/usr/share/fonts/noto/NotoSans-Regular.ttf",
                FontCoverage::Primary,
            ),
            (
                "NotoSansCJK.arch.system.ttc",
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                FontCoverage::Cjk,
            ),
            (
                "NotoSansArabic.arch.system.ttf",
                "/usr/share/fonts/noto/NotoSansArabic-Regular.ttf",
                FontCoverage::Arabic,
            ),
            (
                "NotoSansDevanagari.arch.system.ttf",
                "/usr/share/fonts/noto/NotoSansDevanagari-Regular.ttf",
                FontCoverage::Devanagari,
            ),
            (
                "NotoColorEmoji.arch.system.ttf",
                "/usr/share/fonts/noto/NotoColorEmoji.ttf",
                FontCoverage::Emoji,
            ),
        ],
        "macos" => &[
            (
                "Menlo.system.ttc",
                "/System/Library/Fonts/Menlo.ttc",
                FontCoverage::Primary,
            ),
            (
                "Monaco.system.dfont",
                "/System/Library/Fonts/Monaco.dfont",
                FontCoverage::Primary,
            ),
            (
                "HiraginoSansGB.system.ttc",
                "/System/Library/Fonts/Hiragino Sans GB.ttc",
                FontCoverage::Cjk,
            ),
            (
                "HiraginoSans.system.ttc",
                "/System/Library/Fonts/Hiragino Sans.ttc",
                FontCoverage::Cjk,
            ),
            (
                "AppleSDGothicNeo.system.ttc",
                "/System/Library/Fonts/AppleSDGothicNeo.ttc",
                FontCoverage::Cjk,
            ),
            (
                "ArialUnicode.system.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                FontCoverage::Symbols,
            ),
            (
                "AppleColorEmoji.system.ttc",
                "/System/Library/Fonts/Apple Color Emoji.ttc",
                FontCoverage::Emoji,
            ),
        ],
        _ => &[],
    };
    candidates
        .iter()
        .enumerate()
        .map(|(index, (label, path, coverage))| {
            IndexedFont::file(
                FontKey(u16::try_from(index).expect("reviewed platform candidate count")),
                label,
                *coverage,
                PathBuf::from(path),
            )
        })
        .collect()
}

fn emergency_candidates() -> Vec<IndexedFont> {
    const BASE: u16 = 100;
    vec![
        IndexedFont::embedded_loader(
            FontKey(BASE),
            "NotoSans-Latin.fixture.ttf",
            FontCoverage::Primary,
            true,
            emergency_latin_bytes(),
        ),
        IndexedFont::embedded_loader(
            FontKey(BASE + 1),
            "NotoSansSC-CJK.fixture.ttf",
            FontCoverage::Cjk,
            true,
            emergency_cjk_bytes(),
        ),
        IndexedFont::embedded_loader(
            FontKey(BASE + 2),
            "NotoSansArabic.fixture.ttf",
            FontCoverage::Arabic,
            true,
            emergency_arabic_bytes(),
        ),
        IndexedFont::embedded_loader(
            FontKey(BASE + 3),
            "NotoSansDevanagari.fixture.ttf",
            FontCoverage::Devanagari,
            true,
            emergency_devanagari_bytes(),
        ),
        IndexedFont::embedded_loader(
            FontKey(BASE + 4),
            "NotoSansHebrew.fixture.ttf",
            FontCoverage::Hebrew,
            true,
            emergency_hebrew_bytes(),
        ),
        IndexedFont::embedded_loader(
            FontKey(BASE + 5),
            "NotoSansSymbols2.fixture.ttf",
            FontCoverage::Symbols,
            true,
            emergency_symbols_bytes(),
        ),
        IndexedFont::embedded_loader(
            FontKey(BASE + 6),
            "NotoColorEmoji.fixture.ttf",
            FontCoverage::Emoji,
            true,
            emergency_emoji_bytes(),
        ),
    ]
}

fn emergency_latin_bytes() -> &'static [u8] {
    include_bytes!("../../../tests/fixtures/fonts/NotoSans-Latin.fixture.ttf")
}
fn emergency_cjk_bytes() -> &'static [u8] {
    include_bytes!("../../../tests/fixtures/fonts/NotoSansSC-CJK.fixture.ttf")
}
fn emergency_arabic_bytes() -> &'static [u8] {
    include_bytes!("../../../tests/fixtures/fonts/NotoSansArabic.fixture.ttf")
}
fn emergency_devanagari_bytes() -> &'static [u8] {
    include_bytes!("../../../tests/fixtures/fonts/NotoSansDevanagari.fixture.ttf")
}
fn emergency_hebrew_bytes() -> &'static [u8] {
    include_bytes!("../../../tests/fixtures/fonts/NotoSansHebrew.fixture.ttf")
}
fn emergency_symbols_bytes() -> &'static [u8] {
    include_bytes!("../../../tests/fixtures/fonts/NotoSansSymbols2.fixture.ttf")
}
fn emergency_emoji_bytes() -> &'static [u8] {
    include_bytes!("../../../tests/fixtures/fonts/NotoColorEmoji.fixture.ttf")
}

#[cfg(all(test, feature = "rterm-legacy-0-1"))]
mod tests {
    use super::*;
    use rssh_fonts::{FontConfig, TerminalShaper};

    #[test]
    fn legacy_platform_fonts_consecutive_batches_and_recovery_preserve_state() {
        let mut repository = PlatformFontRepository::production_index_for_os("test");
        let mut catalog = repository.build_catalog(FontCatalogMode::Lazy).unwrap();
        let initial_instance = catalog.incarnation();
        repository.preflight_text("中文😀", &mut catalog).unwrap();
        assert_ne!(catalog.incarnation(), initial_instance);
        assert_eq!(catalog.generation(), 2);
        let first_instance = catalog.incarnation();
        repository.preflight_text("سلام", &mut catalog).unwrap();
        assert_ne!(catalog.incarnation(), first_instance);
        assert_eq!(catalog.generation(), 3);
        assert_eq!(repository.active.len(), 4);
        let order = repository.activation_order.clone();
        let fingerprint = repository.catalog_fingerprint;
        let mut recovered = repository
            .rebuild_catalog_from_active(FontCatalogMode::Lazy)
            .unwrap();
        assert_eq!(recovered.generation(), 3);
        assert_ne!(recovered.incarnation(), catalog.incarnation());
        assert_eq!(repository.activation_order, order);
        assert_eq!(repository.catalog_fingerprint, fingerprint);
        let mut shaper = TerminalShaper::new(FontConfig::new("Noto Sans").with_fallbacks([
            "Noto Sans SC",
            "Noto Color Emoji",
            "Noto Sans Arabic",
        ]));
        let before = shaper.shape_row(&mut catalog, "A中😀سلام").unwrap();
        let after = shaper.shape_row(&mut recovered, "A中😀سلام").unwrap();
        assert!(after.clusters.iter().all(|cluster| !cluster.is_tofu));
        assert_eq!(after.cell_count, before.cell_count);
        assert_eq!(
            after
                .clusters
                .iter()
                .map(|c| &c.font_family)
                .collect::<Vec<_>>(),
            before
                .clusters
                .iter()
                .map(|c| &c.font_family)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn legacy_platform_fonts_invalid_batch_preserves_repository_and_live_ids() {
        let mut repository = PlatformFontRepository::new(
            7,
            vec![
                IndexedFont::embedded(
                    FontKey(1),
                    "latin",
                    FontCoverage::Primary,
                    emergency_latin_bytes(),
                ),
                IndexedFont::embedded(FontKey(2), "cjk", FontCoverage::Cjk, emergency_cjk_bytes()),
                IndexedFont::embedded(FontKey(3), "broken", FontCoverage::Emoji, &[0, 1, 2]),
            ],
        );
        let mut catalog = repository.build_catalog(FontCatalogMode::Lazy).unwrap();
        let identity = (
            catalog.incarnation(),
            catalog.generation(),
            catalog.face_count(),
        );
        let order = repository.activation_order.clone();
        let fingerprint = repository.catalog_fingerprint;
        assert!(repository.preflight_text("中😀", &mut catalog).is_err());
        assert_eq!(
            (
                catalog.incarnation(),
                catalog.generation(),
                catalog.face_count()
            ),
            identity
        );
        assert_eq!(repository.activation_order, order);
        assert_eq!(repository.catalog_fingerprint, fingerprint);
        assert_eq!(repository.active.len(), 1);
        repository.preflight_text("中", &mut catalog).unwrap();
        assert_eq!(catalog.generation(), 2);
        assert_ne!(catalog.incarnation(), identity.0);
    }

    #[test]
    fn legacy_platform_fonts_diagnostics_are_explicitly_unsupported() {
        let repository = PlatformFontRepository::production_index_for_os("test");
        let error = repository.diagnostics().unwrap_err();
        assert!(error.to_string().contains("legacy-0.1"));
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::Unsupported
        );
    }

    #[test]
    fn legacy_platform_fonts_rejects_mismatched_catalog_without_committing() {
        let mut repository = PlatformFontRepository::production_index_for_os("test");
        let live = repository.build_catalog(FontCatalogMode::Lazy).unwrap();
        let order = repository.activation_order.clone();
        let fingerprint = repository.catalog_fingerprint;
        let mut foreign = FontCatalog::new("en-US");
        let identity = (
            foreign.incarnation(),
            foreign.generation(),
            foreign.face_count(),
        );
        let error = repository.preflight_text("中", &mut foreign).unwrap_err();
        assert!(error.to_string().contains("repository epoch"));
        assert_eq!(
            (
                foreign.incarnation(),
                foreign.generation(),
                foreign.face_count()
            ),
            identity
        );
        assert_eq!(repository.generation, live.generation());
        assert_eq!(repository.activation_order, order);
        assert_eq!(repository.catalog_fingerprint, fingerprint);
        assert_eq!(repository.active.len(), 1);
    }

    #[test]
    fn legacy_platform_fonts_late_fallback_converges_and_width_variants_route_to_cjk() {
        let mut repository = PlatformFontRepository::late_missing_fixture();
        let mut catalog = repository.build_catalog(FontCatalogMode::Lazy).unwrap();
        repository.preflight_text("Ａ！中", &mut catalog).unwrap();
        assert_eq!(repository.active.len(), 2);
        let mut shaper =
            TerminalShaper::new(FontConfig::new("Noto Sans").with_fallbacks(["Noto Sans SC"]));
        assert!(
            shaper
                .shape_row(&mut catalog, "中")
                .unwrap()
                .clusters
                .iter()
                .any(|c| c.is_tofu)
        );
        repository
            .activate_missing_glyphs(&['中'], &mut catalog)
            .unwrap();
        assert_eq!(catalog.generation(), 3);
        assert!(
            shaper
                .shape_row(&mut catalog, "中")
                .unwrap()
                .clusters
                .iter()
                .all(|c| !c.is_tofu)
        );
        assert!(!repository.has_pending_fallbacks(&['中']));
        let identity = (catalog.incarnation(), catalog.generation());
        assert_eq!(
            repository.preflight_text("Ａ！中", &mut catalog).unwrap(),
            CatalogActivation::Unchanged
        );
        assert_eq!((catalog.incarnation(), catalog.generation()), identity);
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    fn legacy_platform_fonts_diagnostic_modes_cannot_mutate_the_repository() {
        let mut repository = PlatformFontRepository::production_index_for_os("test");
        for mode in [FontCatalogMode::CurrentCopied, FontCatalogMode::SharedAll] {
            let error = repository.build_catalog(mode).unwrap_err();
            assert_eq!(
                error.downcast_ref::<io::Error>().unwrap().kind(),
                io::ErrorKind::Unsupported
            );
            assert!(repository.active.is_empty());
            assert_eq!(repository.generation, 0);
        }
    }
}

#[cfg(all(test, not(feature = "rterm-legacy-0-1")))]
mod tests {
    use std::path::PathBuf;

    use rssh_fonts::{FontConfig, TerminalShaper};

    use super::*;

    const LATIN: &[u8] = include_bytes!("../../../tests/fixtures/fonts/NotoSans-Latin.fixture.ttf");
    const CJK: &[u8] = include_bytes!("../../../tests/fixtures/fonts/NotoSansSC-CJK.fixture.ttf");
    const ARABIC: &[u8] =
        include_bytes!("../../../tests/fixtures/fonts/NotoSansArabic.fixture.ttf");
    const DEVANAGARI: &[u8] =
        include_bytes!("../../../tests/fixtures/fonts/NotoSansDevanagari.fixture.ttf");
    const HEBREW: &[u8] =
        include_bytes!("../../../tests/fixtures/fonts/NotoSansHebrew.fixture.ttf");
    const SYMBOLS: &[u8] =
        include_bytes!("../../../tests/fixtures/fonts/NotoSansSymbols2.fixture.ttf");
    const EMOJI: &[u8] = include_bytes!("../../../tests/fixtures/fonts/NotoColorEmoji.fixture.ttf");

    fn fixture_repository() -> PlatformFontRepository {
        PlatformFontRepository::new(
            7,
            vec![
                IndexedFont::embedded(FontKey(40), "latin", FontCoverage::Primary, LATIN),
                IndexedFont::embedded(FontKey(30), "cjk", FontCoverage::Cjk, CJK),
                IndexedFont::embedded(FontKey(20), "arabic", FontCoverage::Arabic, ARABIC),
                IndexedFont::embedded(
                    FontKey(10),
                    "devanagari",
                    FontCoverage::Devanagari,
                    DEVANAGARI,
                ),
                IndexedFont::embedded(FontKey(50), "hebrew", FontCoverage::Hebrew, HEBREW),
                IndexedFont::embedded(FontKey(55), "symbols", FontCoverage::Symbols, SYMBOLS),
                IndexedFont::embedded(FontKey(60), "emoji", FontCoverage::Emoji, EMOJI),
            ],
        )
    }

    fn font_config() -> FontConfig {
        FontConfig::new("Noto Sans")
            .with_fallbacks([
                "Noto Sans SC",
                "Noto Sans Arabic",
                "Noto Sans Devanagari",
                "Noto Sans Hebrew",
                "Noto Color Emoji",
            ])
            .with_font_size(16.0)
    }

    #[test]
    fn platform_fonts_windows_index_retains_metadata_but_zero_font_file_bytes() {
        let repository = PlatformFontRepository::production_index_for_os("windows");
        let diagnostics = repository.diagnostics().expect("modern font diagnostics");

        assert_eq!(diagnostics.indexed_source_count, 19);
        assert_eq!(diagnostics.active_source_count, 0);
        assert_eq!(diagnostics.retained_source_bytes, 0);
        assert_eq!(repository.indexed_file_count(), 12);
        assert!(repository.active.is_empty());
        assert!(repository.activation_order.is_empty());
        assert!(
            repository
                .indexed
                .iter()
                .all(IndexedFont::retains_no_font_bytes)
        );
    }

    #[test]
    fn platform_fonts_ascii_preflight_activates_only_primary_once() {
        let mut repository = fixture_repository();
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy ASCII catalog");
        let initial = repository.diagnostics().expect("modern font diagnostics");

        assert_eq!(repository.active_labels(), vec!["latin"]);
        assert_eq!(initial.active_source_count, 1);
        assert_eq!(initial.generation, 1);
        assert_eq!(
            repository
                .preflight_text("ASCII bootstrap", &mut catalog)
                .expect("repeat ASCII preflight"),
            CatalogActivation::Unchanged
        );
        assert_eq!(
            repository.diagnostics().expect("modern font diagnostics"),
            initial
        );
    }

    #[test]
    fn platform_fonts_cjk_and_emoji_activate_minimum_sources_once() {
        let mut repository = fixture_repository();
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy ASCII catalog");

        assert_eq!(
            repository
                .preflight_text("中文😀", &mut catalog)
                .expect("activate CJK and emoji"),
            CatalogActivation::CatalogExpanded {
                previous_generation: 1,
                catalog_generation: 2,
            }
        );
        assert_eq!(repository.active_labels(), vec!["latin", "cjk", "emoji"]);
        let activated = repository.diagnostics().expect("modern font diagnostics");
        assert_eq!(activated.active_source_count, 3);
        assert_eq!(activated.generation, 2);
        let mut shaper = TerminalShaper::new(font_config());
        let shaped_row = shaper
            .shape_row(&mut catalog, "中文😀")
            .expect("shape activated CJK and emoji fixtures");
        assert!(shaped_row.clusters.iter().all(|cluster| !cluster.is_tofu));

        assert_eq!(
            repository
                .preflight_text("😀中文", &mut catalog)
                .expect("repeat preflight"),
            CatalogActivation::Unchanged
        );
        assert_eq!(
            repository.diagnostics().expect("modern font diagnostics"),
            activated
        );
    }

    #[test]
    fn platform_fonts_width_variants_activate_cjk_without_ideographs() {
        for text in ["！", "Ａ", "ｦ", "ﾡ", "￦", "︐", "︰", "﹐", "ᄀ", "ꥠ", "ힰ"] {
            let mut repository = fixture_repository();
            let mut catalog = repository
                .build_catalog(FontCatalogMode::Lazy)
                .expect("build primary catalog");
            let plan = repository.frame_plan(text);
            assert_eq!(plan.required, vec![FontKey(30)], "{text}");
            assert!(!plan.unresolved, "{text}");
            repository
                .activate_plan(&plan, &mut catalog)
                .expect("activate CJK");
            assert_eq!(repository.active_labels(), vec!["latin", "cjk"]);
            assert_eq!(
                repository
                    .preflight_text(text, &mut catalog)
                    .expect("repeat preflight"),
                CatalogActivation::Unchanged,
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn platform_fonts_windows_width_variants_shape_with_lazy_cjk_fallback() {
        // This native-font regression complements the deterministic routing test.
        // Minimal Windows installations may not ship the optional CJK font.
        if !std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc").is_file() {
            return;
        }
        let mut repository = PlatformFontRepository::production_index_for_os("windows");
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build primary production catalog");
        let text = "Ａ！ｦ";
        repository
            .preflight_text(text, &mut catalog)
            .expect("activate width variants");
        let mut shaper = TerminalShaper::new(
            FontConfig::new("Cascadia Mono").with_fallbacks(["Microsoft YaHei"]),
        );
        // Installed CJK families can cover different subsets. This checks
        // repository convergence; GPU continuation is tested in window_gpu.
        for _ in 0..=repository.indexed.len() {
            let row = shaper
                .shape_row(&mut catalog, text)
                .expect("shape width variants");
            let missing = row
                .clusters
                .iter()
                .filter(|cluster| cluster.is_tofu)
                .flat_map(|cluster| row.text[cluster.byte_range.clone()].chars())
                .collect::<Vec<_>>();
            if missing.is_empty() {
                assert!(row.glyphs.iter().all(|glyph| !glyph.is_tofu));
                return;
            }
            assert!(
                matches!(
                    repository
                        .activate_missing_glyphs(&missing, &mut catalog)
                        .expect("activate next installed CJK fallback"),
                    CatalogActivation::CatalogExpanded { .. }
                ),
                "width variants remained missing: {missing:?}; active={:?}",
                repository.active_labels(),
            );
        }
        panic!("width variants did not converge within the font inventory");
    }

    #[test]
    fn platform_fonts_selected_file_is_read_only_once_during_preflight() {
        let cjk_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/fonts/NotoSansSC-CJK.fixture.ttf");
        let mut repository = PlatformFontRepository::new(
            7,
            vec![
                IndexedFont::embedded(FontKey(1), "latin", FontCoverage::Primary, LATIN),
                IndexedFont::file(FontKey(2), "cjk-file", FontCoverage::Cjk, cjk_path),
            ],
        );
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build primary catalog");

        repository
            .preflight_text("中文", &mut catalog)
            .expect("activate selected file");
        repository
            .preflight_text("中文", &mut catalog)
            .expect("reuse selected file");
        let recovered = repository
            .rebuild_catalog_from_active(FontCatalogMode::Lazy)
            .expect("recover from retained active source");

        let selected = repository
            .indexed
            .iter()
            .find(|source| source.key == FontKey(2))
            .expect("indexed CJK file");
        assert_eq!(selected.materialization_count(), 1);
        assert_eq!(recovered.memory_metrics().active_source_count, 2);
    }

    #[test]
    fn platform_fonts_emoji_joiners_do_not_activate_an_unrelated_symbol_source() {
        let mut repository = fixture_repository();
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy ASCII catalog");

        repository
            .preflight_text("👨\u{200d}👩\u{200d}👧☀\u{fe0f}", &mut catalog)
            .expect("activate one emoji source for joiners and variation selectors");

        assert_eq!(repository.active_labels(), vec!["latin", "emoji"]);
    }

    #[test]
    fn platform_fonts_complex_script_fixtures_shape_without_tofu() {
        for (text, expected_label) in [("سلام", "arabic"), ("क्षि", "devanagari"), ("אבג", "hebrew")]
        {
            let mut repository = fixture_repository();
            let mut catalog = repository
                .build_catalog(FontCatalogMode::Lazy)
                .expect("build lazy ASCII catalog");
            repository
                .preflight_text(text, &mut catalog)
                .expect("activate complex-script fixture");
            assert_eq!(repository.active_labels(), vec!["latin", expected_label]);

            let mut shaper = TerminalShaper::new(font_config());
            let row = shaper
                .shape_row(&mut catalog, text)
                .expect("shape complex-script fixture");
            assert!(
                row.clusters.iter().all(|cluster| !cluster.is_tofu),
                "{expected_label} fixture must cover {text}"
            );
        }
    }

    #[test]
    fn platform_fonts_real_shape_missing_activates_the_next_candidate_once() {
        let mut repository = PlatformFontRepository::new(
            7,
            vec![
                IndexedFont::embedded(FontKey(1), "latin", FontCoverage::Primary, LATIN),
                IndexedFont::embedded(
                    FontKey(2),
                    "false-cjk-latin-bytes",
                    FontCoverage::Cjk,
                    LATIN,
                ),
                IndexedFont::embedded(FontKey(3), "real-cjk", FontCoverage::Cjk, CJK),
            ],
        );
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build primary catalog");
        assert_eq!(
            repository
                .preflight_text("中文", &mut catalog)
                .expect("activate first metadata CJK candidate"),
            CatalogActivation::CatalogExpanded {
                previous_generation: 1,
                catalog_generation: 2,
            }
        );
        assert_eq!(
            repository.active_labels(),
            vec!["latin", "false-cjk-latin-bytes"]
        );

        let mut shaper = TerminalShaper::new(font_config());
        let first = shaper
            .shape_row(&mut catalog, "中文")
            .expect("shape against false CJK candidate");
        let missing = first
            .clusters
            .iter()
            .filter(|cluster| cluster.is_tofu)
            .flat_map(|cluster| first.text[cluster.byte_range.clone()].chars())
            .collect::<Vec<_>>();
        assert_eq!(missing, ['中', '文']);

        assert_eq!(
            repository
                .activate_missing_glyphs(&missing, &mut catalog)
                .expect("activate next CJK candidate transactionally"),
            CatalogActivation::CatalogExpanded {
                previous_generation: 2,
                catalog_generation: 3,
            }
        );
        assert_eq!(
            repository.active_labels(),
            vec!["latin", "false-cjk-latin-bytes", "real-cjk"]
        );
        let retried = shaper
            .shape_row(&mut catalog, "中文")
            .expect("reshape after late expansion");
        assert!(retried.clusters.iter().all(|cluster| !cluster.is_tofu));
    }

    #[test]
    fn platform_fonts_missing_script_uses_one_stable_path_without_restart_loop() {
        let mut repository = fixture_repository();
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy ASCII catalog");
        let before = repository.diagnostics().expect("modern font diagnostics");

        for _ in 0..3 {
            assert_eq!(
                repository
                    .preflight_text("\u{10ffff}", &mut catalog)
                    .expect("stable missing-glyph preflight"),
                CatalogActivation::StableMissingGlyph
            );
            assert_eq!(
                repository.diagnostics().expect("modern font diagnostics"),
                before
            );
        }
    }

    #[test]
    fn platform_fonts_invalid_late_batch_preserves_repository_catalog_and_font_ids() {
        static INVALID_FONT: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7];
        let mut repository = PlatformFontRepository::new(
            7,
            vec![
                IndexedFont::embedded(FontKey(1), "latin", FontCoverage::Primary, LATIN),
                IndexedFont::embedded(FontKey(2), "arabic", FontCoverage::Arabic, ARABIC),
                IndexedFont::embedded(FontKey(3), "hebrew", FontCoverage::Hebrew, HEBREW),
                IndexedFont::embedded(FontKey(4), "cjk", FontCoverage::Cjk, CJK),
                IndexedFont::embedded(
                    FontKey(5),
                    "invalid-emoji",
                    FontCoverage::Emoji,
                    INVALID_FONT,
                ),
            ],
        );
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build primary catalog");
        repository
            .preflight_text("سلامאבג", &mut catalog)
            .expect("activate valid complex-script batch");
        let mut shaper = TerminalShaper::new(font_config());
        let before_shape = shaper
            .shape_row(&mut catalog, "سلامאבג")
            .expect("shape stable multi-script row");
        assert!(before_shape.clusters.iter().all(|cluster| !cluster.is_tofu));
        let before_font_ids = before_shape
            .clusters
            .iter()
            .map(|cluster| cluster.font_id)
            .collect::<Vec<_>>();
        let before_repository = repository.diagnostics().expect("modern font diagnostics");
        let before_generation = catalog.generation();
        let before_faces = catalog.face_count();
        let before_memory = catalog.memory_metrics();

        let invalid_plan = repository.frame_plan("中😀");
        assert!(
            repository
                .activate_plan(&invalid_plan, &mut catalog)
                .is_err()
        );

        assert_eq!(
            repository.diagnostics().expect("modern font diagnostics"),
            before_repository
        );
        assert_eq!(catalog.generation(), before_generation);
        assert_eq!(catalog.face_count(), before_faces);
        assert_eq!(catalog.memory_metrics(), before_memory);
        let after_shape = shaper
            .shape_row(&mut catalog, "سلامאבג")
            .expect("reshape after rejected batch");
        assert!(after_shape.clusters.iter().all(|cluster| !cluster.is_tofu));
        assert_eq!(
            after_shape
                .clusters
                .iter()
                .map(|cluster| cluster.font_id)
                .collect::<Vec<_>>(),
            before_font_ids
        );
    }

    #[test]
    fn platform_fonts_activation_commits_ordered_fingerprint_and_generation_atomically() {
        let mut repository = fixture_repository();
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy ASCII catalog");
        let before = repository.diagnostics().expect("modern font diagnostics");
        let plan = repository.frame_plan("😀中文");

        assert_eq!(plan.catalog_fingerprint, before.catalog_fingerprint);
        repository
            .activate_plan(&plan, &mut catalog)
            .expect("activate ordered plan");
        let after = repository.diagnostics().expect("modern font diagnostics");
        assert_eq!(after.generation, before.generation + 1);
        assert_ne!(after.catalog_fingerprint, before.catalog_fingerprint);
        assert_eq!(repository.active_labels(), vec!["latin", "emoji", "cjk"]);

        let invalid_path = PathBuf::from("definitely-missing-stage7-font.ttf");
        repository.indexed.push(IndexedFont::file(
            FontKey(70),
            "missing-cjk",
            FontCoverage::Cjk,
            invalid_path,
        ));
        repository.active.remove(&FontKey(30));
        repository
            .activation_order
            .retain(|key| *key != FontKey(30));
        let stable = repository.diagnostics().expect("modern font diagnostics");
        let invalid = FrameFontPlan {
            required: vec![FontKey(70)],
            catalog_fingerprint: stable.catalog_fingerprint,
            unresolved: false,
        };
        assert!(repository.activate_plan(&invalid, &mut catalog).is_err());
        assert_eq!(
            repository.diagnostics().expect("modern font diagnostics"),
            stable
        );
    }

    #[test]
    fn platform_fonts_active_fingerprint_hashes_small_ordered_source_digests() {
        fn push_field(envelope: &mut Vec<u8>, field: &[u8]) {
            envelope.extend_from_slice(
                &u64::try_from(field.len())
                    .expect("field length")
                    .to_le_bytes(),
            );
            envelope.extend_from_slice(field);
        }

        let first_key = FontKey(41);
        let second_key = FontKey(7);
        let first = FontSource::new("first", LATIN.to_vec());
        let second = FontSource::new("second", CJK.to_vec());
        let active = BTreeMap::from([(first_key, first.clone()), (second_key, second.clone())]);
        let order = [first_key, second_key];

        let mut envelope = Vec::new();
        envelope.extend_from_slice(b"rssh-platform-font-active-v2\0");
        envelope.extend_from_slice(&7_u32.to_le_bytes());
        envelope.extend_from_slice(&2_u64.to_le_bytes());
        for (key, source) in [(first_key, &first), (second_key, &second)] {
            push_field(&mut envelope, &key.0.to_le_bytes());
            push_field(&mut envelope, source.label.as_bytes());
            push_field(
                &mut envelope,
                &terminal_bytes_content_digest(source.bytes()),
            );
        }
        let expected = terminal_bytes_content_digest(&envelope);

        assert_eq!(active_fingerprint(7, &order, &active), expected);
        assert_ne!(
            active_fingerprint(7, &order, &active),
            active_fingerprint(7, &[second_key, first_key], &active),
            "activation order must not be replaced by BTreeMap key order"
        );
    }

    #[test]
    fn platform_font_inventory_fingerprint_is_order_independent_and_binds_identity_and_content() {
        let first = [("latin", LATIN), ("cjk", CJK), ("emoji", EMOJI)];
        let reordered = [("emoji", EMOJI), ("latin", LATIN), ("cjk", CJK)];
        let changed_label = [("latin-renamed", LATIN), ("cjk", CJK), ("emoji", EMOJI)];
        let changed_content = [("latin", ARABIC), ("cjk", CJK), ("emoji", EMOJI)];

        assert_eq!(
            font_inventory_fingerprint(first),
            font_inventory_fingerprint(reordered),
            "candidate order must not affect the inventory cohort"
        );
        assert_ne!(
            font_inventory_fingerprint(first),
            font_inventory_fingerprint(changed_label),
            "stable source identity must be bound"
        );
        assert_ne!(
            font_inventory_fingerprint(first),
            font_inventory_fingerprint(changed_content),
            "source content must be bound"
        );
    }

    #[test]
    fn platform_fonts_plans_probe_each_required_coverage_only_once() {
        let mut repository = fixture_repository();
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy primary catalog");
        let before_frame = repository.availability_probe_count();
        let text = format!("{}{}", "😀".repeat(512), "中".repeat(512));
        let plan = repository.frame_plan(&text);
        assert_eq!(plan.required, [FontKey(60), FontKey(30)]);
        assert_eq!(
            repository.availability_probe_count() - before_frame,
            2,
            "frame planning must select and probe once per new coverage"
        );
        repository
            .activate_plan(&plan, &mut catalog)
            .expect("activate deterministic frame plan");
        assert_eq!(repository.active_labels(), ["latin", "emoji", "cjk"]);

        let before_missing = repository.availability_probe_count();
        let missing = "سلام"
            .repeat(256)
            .chars()
            .chain("אבג".repeat(256).chars())
            .collect::<Vec<_>>();
        repository
            .activate_missing_glyphs(&missing, &mut catalog)
            .expect("activate deterministic missing-glyph plan");
        assert_eq!(
            repository.availability_probe_count() - before_missing,
            2,
            "late missing planning must select and probe once per new coverage"
        );
        assert_eq!(
            repository.active_labels(),
            ["latin", "emoji", "cjk", "arabic", "hebrew"]
        );
    }

    #[test]
    fn platform_fonts_recovery_reuses_repository_without_retained_byte_multiplication() {
        let mut repository = fixture_repository();
        let catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy ASCII catalog");
        let mut catalog = repository
            .preflight_text("中文😀", &mut { catalog })
            .and_then(|_| repository.rebuild_catalog_from_active(FontCatalogMode::Lazy))
            .expect("rebuild from app-owned active repository");
        let before = repository.diagnostics().expect("modern font diagnostics");
        let rebuilt = repository
            .rebuild_catalog_from_active(FontCatalogMode::Lazy)
            .expect("device-loss catalog rebuild");

        assert_eq!(
            repository.diagnostics().expect("modern font diagnostics"),
            before
        );
        assert_eq!(rebuilt.memory_metrics().active_source_count, 3);
        assert_eq!(catalog.memory_metrics().active_source_count, 3);
        catalog
            .load_sources(std::iter::empty())
            .expect("keep comparison catalog live");
    }

    #[test]
    fn platform_fonts_lazy_recovery_preserves_generation_builds_and_next_activation() {
        let mut repository = fixture_repository();
        let mut catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy primary catalog");
        repository
            .preflight_text("中文😀", &mut catalog)
            .expect("activate one fallback batch");
        let before = repository.diagnostics().expect("modern font diagnostics");
        assert_eq!(before.generation, 2);
        assert_eq!(before.catalog_builds, 2);
        let mut shaper = TerminalShaper::new(font_config());
        let before_row = shaper
            .shape_row(&mut catalog, "A中😀")
            .expect("shape active sources before recovery");
        assert!(before_row.clusters.iter().all(|cluster| !cluster.is_tofu));

        let mut recovered = repository
            .rebuild_catalog_from_active(FontCatalogMode::Lazy)
            .expect("rebuild with the repository epoch");
        assert_eq!(recovered.generation(), before.generation);
        assert_eq!(
            recovered.memory_metrics().catalog_builds,
            before.catalog_builds
        );
        assert_eq!(
            repository.diagnostics().expect("modern font diagnostics"),
            before
        );
        let recovered_row = shaper
            .shape_row(&mut recovered, "A中😀")
            .expect("shape active sources after recovery");
        assert!(
            recovered_row
                .clusters
                .iter()
                .all(|cluster| !cluster.is_tofu)
        );
        assert_eq!(
            recovered_row
                .clusters
                .iter()
                .map(|cluster| cluster.font_family.as_str())
                .collect::<Vec<_>>(),
            before_row
                .clusters
                .iter()
                .map(|cluster| cluster.font_family.as_str())
                .collect::<Vec<_>>()
        );
        let before_ids = before_row
            .clusters
            .iter()
            .map(|cluster| cluster.font_id)
            .collect::<Vec<_>>();
        let recovered_ids = recovered_row
            .clusters
            .iter()
            .map(|cluster| cluster.font_id)
            .collect::<Vec<_>>();
        for left in 0..before_ids.len() {
            for right in 0..before_ids.len() {
                assert_eq!(
                    before_ids[left] == before_ids[right],
                    recovered_ids[left] == recovered_ids[right],
                    "recovery must preserve face-selection identity relationships"
                );
            }
        }
        assert!(
            recovered_ids
                .iter()
                .all(|font_id| font_id.catalog_generation() == before.generation)
        );

        assert_eq!(
            repository
                .preflight_text("سلام", &mut recovered)
                .expect("activate monotonically after recovery"),
            CatalogActivation::CatalogExpanded {
                previous_generation: 2,
                catalog_generation: 3,
            }
        );
        let after = repository.diagnostics().expect("modern font diagnostics");
        assert_eq!(after.generation, 3);
        assert_eq!(after.catalog_builds, 3);
        assert_eq!(recovered.generation(), after.generation);
        assert_eq!(
            recovered.memory_metrics().catalog_builds,
            after.catalog_builds
        );
    }

    #[test]
    fn platform_fonts_recovery_rejects_impossible_repository_epochs() {
        let mut zero_epoch = fixture_repository();
        zero_epoch
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build active repository");
        zero_epoch.generation = 0;
        zero_epoch.catalog_builds = 0;
        assert!(
            zero_epoch
                .rebuild_catalog_from_active(FontCatalogMode::Lazy)
                .is_err()
        );

        let mut excess_epoch = fixture_repository();
        excess_epoch
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build active repository");
        excess_epoch.generation = 2;
        excess_epoch.catalog_builds = 2;
        assert!(
            excess_epoch
                .rebuild_catalog_from_active(FontCatalogMode::Lazy)
                .is_err()
        );
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    fn platform_fonts_current_copied_recovery_preserves_legacy_build_shape() {
        let mut repository = fixture_repository();
        let initial = repository
            .build_catalog(FontCatalogMode::CurrentCopied)
            .expect("build legacy copied catalog");
        let recovered = repository
            .rebuild_catalog_from_active(FontCatalogMode::CurrentCopied)
            .expect("rebuild legacy copied catalog");

        assert_eq!(recovered.generation(), initial.generation());
        assert_eq!(recovered.memory_metrics(), initial.memory_metrics());
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    fn platform_fonts_record_the_actual_initial_catalog_batch_independently() {
        fn mixed_repository() -> PlatformFontRepository {
            PlatformFontRepository::new(
                7,
                vec![
                    IndexedFont::embedded_loader(
                        FontKey(1),
                        "latin",
                        FontCoverage::Primary,
                        true,
                        LATIN,
                    ),
                    IndexedFont::embedded_loader(FontKey(2), "cjk", FontCoverage::Cjk, false, CJK),
                    IndexedFont::embedded_loader(
                        FontKey(3),
                        "emoji",
                        FontCoverage::Emoji,
                        false,
                        EMOJI,
                    ),
                ],
            )
        }

        let mut copied = mixed_repository();
        copied
            .build_catalog(FontCatalogMode::CurrentCopied)
            .expect("build copied catalog");
        let copied_diagnostics = copied.diagnostics().expect("modern font diagnostics");
        assert_eq!(copied_diagnostics.initial_catalog_source_count, 1);
        assert_eq!(copied_diagnostics.active_source_count, 3);
        assert_eq!(copied_diagnostics.catalog_builds, 3);

        let mut shared = mixed_repository();
        shared
            .build_catalog(FontCatalogMode::SharedAll)
            .expect("build shared catalog");
        let shared_diagnostics = shared.diagnostics().expect("modern font diagnostics");
        assert_eq!(shared_diagnostics.initial_catalog_source_count, 3);
        assert_eq!(shared_diagnostics.active_source_count, 3);
        assert_eq!(shared_diagnostics.catalog_builds, 1);

        let mut lazy = mixed_repository();
        let mut lazy_catalog = lazy
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy catalog");
        assert_eq!(
            lazy.diagnostics()
                .expect("modern font diagnostics")
                .initial_catalog_source_count,
            1
        );
        lazy.preflight_text("中文", &mut lazy_catalog)
            .expect("activate one lazy source");
        let activated = lazy.diagnostics().expect("modern font diagnostics");
        assert_eq!(activated.initial_catalog_source_count, 1);
        assert_eq!(activated.active_source_count, 2);
        assert_eq!(activated.catalog_builds, 2);
        lazy.rebuild_catalog_from_active(FontCatalogMode::Lazy)
            .expect("rebuild repository epoch");
        assert_eq!(
            lazy.diagnostics()
                .expect("modern font diagnostics")
                .initial_catalog_source_count,
            1
        );
    }

    #[test]
    fn platform_fonts_diagnostics_expose_only_counts_and_irreversible_digests() {
        let repository = PlatformFontRepository::production_index_for_os("windows");
        let rendered = format!(
            "{:?}",
            repository.diagnostics().expect("modern font diagnostics")
        );

        assert!(!rendered.contains(r"C:\Windows\Fonts"));
        assert!(!rendered.to_ascii_lowercase().contains("path"));
        assert!(rendered.contains("index_fingerprint"));
        assert!(rendered.contains("catalog_fingerprint"));
    }

    #[test]
    fn linux_font_inventory_supports_arch_and_debian_install_locations() {
        let candidates = platform_candidates("linux");
        for (path, coverage) in [
            (
                "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
                FontCoverage::Primary,
            ),
            (
                "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
                FontCoverage::Primary,
            ),
            (
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                FontCoverage::Cjk,
            ),
            (
                "/usr/share/fonts/noto/NotoSansArabic-Regular.ttf",
                FontCoverage::Arabic,
            ),
            (
                "/usr/share/fonts/noto/NotoSansDevanagari-Regular.ttf",
                FontCoverage::Devanagari,
            ),
            (
                "/usr/share/fonts/noto/NotoColorEmoji.ttf",
                FontCoverage::Emoji,
            ),
            (
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                FontCoverage::Cjk,
            ),
        ] {
            assert!(candidates.iter().any(|candidate| {
                candidate.coverage == coverage && !candidate.emergency
                    && matches!(&candidate.locator, IndexedFontLocator::File(found) if found == &PathBuf::from(path))
            }), "missing native font candidate: {path}");
        }
    }

    #[test]
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    fn production_fonts_cover_desktop_chinese_beyond_emergency_subset() {
        let mut repository = PlatformFontRepository::production_index();
        let mut catalog = repository.build_catalog(FontCatalogMode::Lazy).unwrap();
        let text = "终端输入测试你好世界";
        repository.preflight_text(text, &mut catalog).unwrap();
        let mut shaper = TerminalShaper::new(crate::window_cpu::font_config(None, 17.0));
        let row = shaper.shape_row(&mut catalog, text).unwrap();
        assert!(
            row.clusters.iter().all(|cluster| !cluster.is_tofu),
            "the desktop requires an installed full CJK font; the emergency subset is insufficient"
        );
        assert_eq!(row.cell_count, text.chars().count() * 2);
        assert!(
            repository.indexed.iter().any(|source| {
                source.coverage == FontCoverage::Cjk
                    && !source.emergency
                    && repository.active.contains_key(&source.key)
            }),
            "CJK coverage must come from an installed font"
        );
    }

    #[test]
    fn platform_fonts_production_default_is_lazy_after_gate_zero() {
        assert_eq!(production_font_catalog_mode(), FontCatalogMode::Lazy);
    }

    #[test]
    fn production_fonts_cover_ascii_cjk_emoji_arabic_devanagari_and_hebrew_without_tofu() {
        let mut repository = fixture_repository();
        let mut catalog = repository
            .build_catalog(production_font_catalog_mode())
            .expect("build production catalog");
        let text = "ASCII 中文 😀 سلام क्षि אבג";

        assert_eq!(production_font_catalog_mode(), FontCatalogMode::Lazy);
        assert_eq!(
            repository
                .preflight_text(text, &mut catalog)
                .expect("activate production fixture coverage"),
            CatalogActivation::CatalogExpanded {
                previous_generation: 1,
                catalog_generation: 2,
            }
        );
        let mut shaper = TerminalShaper::new(font_config());
        let row = shaper
            .shape_row(&mut catalog, text)
            .expect("shape production fixture coverage");
        assert!(row.clusters.iter().all(|cluster| !cluster.is_tofu));
        assert!(row.glyphs.iter().all(|glyph| !glyph.is_tofu));
        assert_eq!(row.catalog_generation, catalog.generation());
    }

    #[test]
    fn production_fonts_missing_font_is_one_stable_emergency_glyph() {
        let mut repository = fixture_repository();
        let mut catalog = repository
            .build_catalog(production_font_catalog_mode())
            .expect("build production catalog");
        let before = repository.diagnostics().expect("modern font diagnostics");

        for _ in 0..2 {
            assert_eq!(
                repository
                    .preflight_text("\u{10ffff}", &mut catalog)
                    .expect("stable production missing glyph"),
                CatalogActivation::StableMissingGlyph
            );
            let mut shaper = TerminalShaper::new(font_config());
            let row = shaper
                .shape_row(&mut catalog, "\u{10ffff}")
                .expect("shape stable production missing glyph");
            assert_eq!(
                row.clusters
                    .iter()
                    .filter(|cluster| cluster.is_tofu)
                    .count(),
                1
            );
            assert_eq!(row.glyphs.iter().filter(|glyph| glyph.is_tofu).count(), 1);
            assert_eq!(row.catalog_generation, catalog.generation());
            assert_eq!(
                repository.diagnostics().expect("modern font diagnostics"),
                before
            );
        }
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    fn platform_fonts_font_mode_shared_all_uses_one_catalog_allocation_per_source() {
        let mut copied_repository = fixture_repository();
        let copied = copied_repository
            .build_catalog(FontCatalogMode::CurrentCopied)
            .expect("build copied diagnostic baseline");
        let mut shared_repository = fixture_repository();
        let shared = shared_repository
            .build_catalog(FontCatalogMode::SharedAll)
            .expect("build shared diagnostic catalog");

        assert_eq!(
            copied.memory_metrics().retained_source_bytes,
            shared
                .memory_metrics()
                .retained_source_bytes
                .saturating_mul(2)
        );
        assert_eq!(
            shared.memory_metrics().retained_source_bytes,
            shared_repository
                .diagnostics()
                .expect("modern font diagnostics")
                .retained_source_bytes
        );
        for source_index in 0..shared.memory_metrics().active_source_count {
            assert!(shared.diagnostic_fontdb_shares_source_allocation(source_index));
        }
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    fn platform_fonts_font_mode_lazy_keeps_only_primary_in_one_shared_allocation() {
        let mut repository = fixture_repository();
        let lazy = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("build lazy diagnostic catalog");
        let diagnostics = repository.diagnostics().expect("modern font diagnostics");

        assert_eq!(lazy.memory_metrics().active_source_count, 1);
        assert_eq!(diagnostics.active_source_count, 1);
        assert!(diagnostics.indexed_source_count > diagnostics.active_source_count);
        assert_eq!(
            lazy.memory_metrics().retained_source_bytes,
            diagnostics.retained_source_bytes
        );
        assert!(lazy.diagnostic_fontdb_shares_source_allocation(0));
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    fn platform_fonts_font_mode_recovery_preserves_shared_ownership_without_duplication() {
        for mode in [FontCatalogMode::SharedAll, FontCatalogMode::Lazy] {
            let mut repository = fixture_repository();
            let initial = repository.build_catalog(mode).expect("build proof catalog");
            let recovered = repository
                .rebuild_catalog_from_active(mode)
                .expect("recover proof catalog");
            assert_eq!(recovered.memory_metrics(), initial.memory_metrics());
            assert_eq!(
                recovered.memory_metrics().retained_source_bytes,
                repository
                    .diagnostics()
                    .expect("modern font diagnostics")
                    .retained_source_bytes
            );
            for source_index in 0..recovered.memory_metrics().active_source_count {
                assert!(recovered.diagnostic_fontdb_shares_source_allocation(source_index));
            }
        }
    }
}
