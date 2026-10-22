use std::{error::Error, io};

use rterm_fonts::{FontCatalog, FontSource};

#[cfg(feature = "rterm-legacy-0-1")]
pub(crate) fn unsupported_diagnostics() -> Box<dyn Error> {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "R-Term legacy-0.1 diagnostic-tools are unsupported",
    )
    .into()
}

// The frozen API only exposes single-source mutation. Reconstruct the committed
// logical epoch on a private catalog; never replay mutations on the live one.
// Source order is significant, including when a batch added several sources.
pub(crate) fn rebuild_catalog(
    ordered: &[FontSource],
    epoch: u64,
) -> Result<FontCatalog, Box<dyn Error>> {
    let builds = usize::try_from(epoch)
        .ok()
        .filter(|builds| *builds > 0 && *builds <= ordered.len())
        .ok_or_else(|| io::Error::other("legacy font epoch cannot be reconstructed"))?;
    let initial_count = ordered.len() - (builds - 1);
    let mut candidate = FontCatalog::from_sources("en-US", ordered[..initial_count].to_vec())?;
    for source in &ordered[initial_count..] {
        candidate.load_source(source.clone())?;
    }
    if candidate.generation() != epoch {
        return Err(io::Error::other("legacy font catalog epoch mismatch").into());
    }
    Ok(candidate)
}

// `ordered` must contain the repository's active sources followed by the new
// batch. Publication changes incarnation as well as generation; consumers must
// invalidate instance-scoped font IDs. These epochs are not memory metrics.
pub(crate) fn replace_catalog(
    live: &mut FontCatalog,
    ordered: &[FontSource],
) -> Result<u64, Box<dyn Error>> {
    let epoch = live
        .generation()
        .checked_add(1)
        .ok_or_else(|| io::Error::other("legacy font generation exhausted"))?;
    let candidate = rebuild_catalog(ordered, epoch)?;
    *live = candidate;
    Ok(epoch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rterm_fonts::{FontCatalog, FontConfig, FontSource, TerminalShaper};

    fn sources() -> Vec<FontSource> {
        vec![
            FontSource::new(
                "latin",
                include_bytes!("../../../tests/fixtures/fonts/NotoSans-Latin.fixture.ttf").to_vec(),
            ),
            FontSource::new(
                "cjk",
                include_bytes!("../../../tests/fixtures/fonts/NotoSansSC-CJK.fixture.ttf").to_vec(),
            ),
            FontSource::new(
                "arabic",
                include_bytes!("../../../tests/fixtures/fonts/NotoSansArabic.fixture.ttf").to_vec(),
            ),
        ]
    }

    #[test]
    fn legacy_fonts_batch_advances_one_epoch_and_changes_incarnation() {
        let ordered = sources();
        let mut live = FontCatalog::from_sources("en-US", [ordered[0].clone()]).unwrap();
        let incarnation = live.incarnation();
        assert_eq!(replace_catalog(&mut live, &ordered).unwrap(), 2);
        assert_eq!(live.generation(), 2);
        assert_ne!(live.incarnation(), incarnation);
        assert_eq!(live.face_count(), 3);
    }

    #[test]
    fn legacy_fonts_bad_tail_never_partially_updates_live_catalog() {
        let mut ordered = sources();
        let mut live = FontCatalog::from_sources("en-US", [ordered[0].clone()]).unwrap();
        let identity = (live.incarnation(), live.generation(), live.face_count());
        ordered.push(FontSource::new("broken-tail", vec![0, 1, 2]));
        assert!(replace_catalog(&mut live, &ordered).is_err());
        assert_eq!(
            (live.incarnation(), live.generation(), live.face_count()),
            identity
        );
    }

    #[test]
    fn legacy_fonts_recovery_preserves_epoch_but_not_instance() {
        let ordered = sources();
        let first = rebuild_catalog(&ordered, 2).unwrap();
        let second = rebuild_catalog(&ordered, 2).unwrap();
        assert_eq!(first.generation(), second.generation());
        assert_ne!(first.incarnation(), second.incarnation());
        assert_eq!(first.face_count(), second.face_count());
    }

    #[test]
    fn legacy_fonts_recovery_invalidates_cached_ids_at_the_same_epoch() {
        let ordered = sources();
        let mut live = rebuild_catalog(&ordered, 2).unwrap();
        let mut shaper = TerminalShaper::new(FontConfig::new("Noto Sans"));
        let before = shaper.shape_row(&mut live, "ASCII").unwrap();
        let old_id = before.glyphs[0].font_id;
        live = rebuild_catalog(&ordered, 2).unwrap();
        let after = shaper.shape_row(&mut live, "ASCII").unwrap();
        assert_eq!(before.catalog_generation, after.catalog_generation);
        assert_ne!(old_id, after.glyphs[0].font_id);
        assert!(
            after
                .glyphs
                .iter()
                .all(|glyph| { glyph.font_id.catalog_incarnation() == live.incarnation() })
        );
    }

    #[test]
    fn legacy_fonts_replay_preserves_source_order_at_every_epoch() {
        let mut ordered = sources();
        ordered.reverse();
        let families = |catalog: &mut FontCatalog| {
            catalog
                .font_system_mut()
                .db()
                .faces()
                .map(|face| face.families.clone())
                .collect::<Vec<_>>()
        };
        let mut direct = FontCatalog::from_sources("en-US", ordered.clone()).unwrap();
        let expected = families(&mut direct);
        for epoch in 1..=3 {
            let mut replayed = rebuild_catalog(&ordered, epoch).unwrap();
            assert_eq!(families(&mut replayed), expected);
        }
    }

    #[test]
    fn legacy_fonts_rejects_unreconstructable_epochs() {
        let ordered = sources();
        for epoch in [0, 4, u64::MAX] {
            assert!(rebuild_catalog(&ordered, epoch).is_err(), "epoch {epoch}");
        }
        assert!(rebuild_catalog(&[], 1).is_err());
    }

    #[test]
    fn legacy_fonts_late_cjk_reuses_shaper_without_stale_tofu() {
        let ordered = sources();
        let mut live = rebuild_catalog(&ordered[..1], 1).unwrap();
        let mut shaper =
            TerminalShaper::new(FontConfig::new("Noto Sans").with_fallbacks(["Noto Sans SC"]));
        let before = shaper.shape_row(&mut live, "中文").unwrap();
        assert!(before.clusters.iter().any(|cluster| cluster.is_tofu));
        replace_catalog(&mut live, &ordered[..2]).unwrap();
        let after = shaper.shape_row(&mut live, "中文").unwrap();
        assert!(after.clusters.iter().all(|cluster| !cluster.is_tofu));
        assert_eq!(after.cell_count, before.cell_count);
    }
}
