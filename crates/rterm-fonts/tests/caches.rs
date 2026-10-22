use std::path::{Path, PathBuf};

use cosmic_text::Family;
use rterm_fonts::{
    FontCatalog, FontConfig, FontSource, RasterCache, RasterCacheConfig, RasterContent,
    RasterFallback, RasterFlags, RasterRequest, ShapedGlyph, ShapedRow, TerminalShaper,
};

const LATIN: &str = "NotoSans-Latin.fixture.ttf";
const EMOJI: &str = "NotoColorEmoji.fixture.ttf";
const CJK: &str = "NotoSansSC-CJK.fixture.ttf";
const ARABIC: &str = "NotoSansArabic.fixture.ttf";
const DEVANAGARI: &str = "NotoSansDevanagari.fixture.ttf";

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/fonts")
        .join(name)
}

fn source(name: &str) -> FontSource {
    FontSource::new(
        name,
        std::fs::read(fixture_path(name)).expect("read fixture"),
    )
}

fn catalog(names: &[&str]) -> FontCatalog {
    FontCatalog::from_sources("en-US", names.iter().map(|name| source(name)))
        .expect("fixture catalog")
}

#[test]
fn transactional_batch_commits_one_build_and_one_generation() {
    let mut catalog = catalog(&[LATIN]);
    let before = catalog.memory_metrics();
    let expected_retained_bytes = [LATIN, CJK, EMOJI]
        .iter()
        .map(|name| {
            usize::try_from(
                std::fs::metadata(fixture_path(name))
                    .expect("fixture metadata")
                    .len(),
            )
            .expect("fixture length fits usize")
        })
        .sum::<usize>();
    let generation = catalog
        .load_sources([source(CJK), source(EMOJI)])
        .expect("load valid batch");

    assert_eq!(generation, 2);
    assert_eq!(catalog.generation(), 2);
    assert_eq!(catalog.face_count(), 3);
    assert_eq!(catalog.memory_metrics().active_source_count, 3);
    assert_eq!(
        catalog.memory_metrics().retained_source_bytes,
        expected_retained_bytes
    );
    assert_eq!(
        catalog.memory_metrics().catalog_builds,
        before.catalog_builds + 1
    );
}

#[test]
fn transactional_batch_rejects_all_sources_without_mutating_catalog() {
    let mut catalog = catalog(&[LATIN, CJK, EMOJI]);
    catalog
        .font_system_mut()
        .db_mut()
        .set_monospace_family("transaction-sentinel");
    let config = FontConfig::new("Noto Sans").with_fallbacks(["Noto Sans SC", "Noto Color Emoji"]);
    let mut before_shaper = TerminalShaper::new(config.clone());
    let before_font_ids: Vec<_> = before_shaper
        .shape_row(&mut catalog, "A界😀")
        .expect("shape before rejected batch")
        .glyphs
        .iter()
        .map(|glyph| glyph.font_id)
        .collect();
    let before_generation = catalog.generation();
    let before_face_count = catalog.face_count();
    let before_metrics = catalog.memory_metrics();
    #[cfg(feature = "diagnostic-tools")]
    let before_fingerprint = catalog.diagnostic_ordered_fingerprint();
    #[cfg(feature = "diagnostic-tools")]
    let before_records = catalog.diagnostic_face_records_fingerprint();

    let error = catalog
        .load_sources([
            source(CJK),
            FontSource::new("invalid batch member", vec![0, 1, 2, 3]),
        ])
        .expect_err("invalid source rejects whole batch");

    assert_eq!(
        error.to_string(),
        "invalid font source invalid batch member"
    );
    assert_eq!(catalog.generation(), before_generation);
    assert_eq!(catalog.face_count(), before_face_count);
    assert_eq!(catalog.memory_metrics(), before_metrics);
    assert_eq!(
        catalog
            .font_system_mut()
            .db()
            .family_name(&Family::Monospace),
        "transaction-sentinel"
    );
    let mut after_shaper = TerminalShaper::new(config);
    let after_font_ids: Vec<_> = after_shaper
        .shape_row(&mut catalog, "A界😀")
        .expect("shape after rejected batch")
        .glyphs
        .iter()
        .map(|glyph| glyph.font_id)
        .collect();
    assert_eq!(after_font_ids, before_font_ids);
    #[cfg(feature = "diagnostic-tools")]
    assert_eq!(catalog.diagnostic_ordered_fingerprint(), before_fingerprint);
    #[cfg(feature = "diagnostic-tools")]
    assert_eq!(
        catalog.diagnostic_face_records_fingerprint(),
        before_records
    );
}

#[test]
fn transactional_batch_treats_an_empty_batch_as_a_no_op() {
    let mut catalog = catalog(&[LATIN]);
    let before_generation = catalog.generation();
    let before_metrics = catalog.memory_metrics();

    let generation = catalog
        .load_sources(std::iter::empty())
        .expect("empty batch is valid");

    assert_eq!(generation, before_generation);
    assert_eq!(catalog.generation(), before_generation);
    assert_eq!(catalog.memory_metrics(), before_metrics);
}

#[test]
fn shape_cache_is_lru_and_strictly_byte_bounded() {
    let mut catalog = catalog(&[LATIN]);
    let mut shaper = TerminalShaper::with_cache_budget(FontConfig::new("Noto Sans"), 1 << 20);
    shaper.shape_row(&mut catalog, "aaaa").expect("measure");
    let one_entry = shaper.cache_metrics().current_bytes;
    assert!(one_entry > 0);
    shaper.set_cache_budget(0);
    shaper.set_cache_budget(one_entry * 2);

    shaper.shape_row(&mut catalog, "aaaa").expect("a");
    shaper.shape_row(&mut catalog, "bbbb").expect("b");
    shaper.shape_row(&mut catalog, "aaaa").expect("a hit");
    shaper.shape_row(&mut catalog, "cccc").expect("c evicts b");
    shaper.shape_row(&mut catalog, "aaaa").expect("a retained");
    let misses_before_b = shaper.cache_stats().misses;
    shaper
        .shape_row(&mut catalog, "bbbb")
        .expect("b was evicted");
    assert_eq!(shaper.cache_stats().misses, misses_before_b + 1);

    let metrics = shaper.cache_metrics();
    assert_eq!(metrics.hits, 2);
    assert_eq!(metrics.evictions, 3);
    assert!(metrics.current_bytes <= metrics.budget_bytes);
    assert!(metrics.peak_bytes <= metrics.budget_bytes);
    assert!(metrics.current_entries > 0);
}

#[test]
fn zero_and_oversize_shape_budgets_bypass_without_retaining_rows() {
    let mut catalog = catalog(&[LATIN]);
    let mut zero = TerminalShaper::with_cache_budget(FontConfig::new("Noto Sans"), 0);
    zero.shape_row(&mut catalog, "small").expect("shape");
    zero.shape_row(&mut catalog, "small").expect("shape again");
    assert_eq!(zero.cache_metrics().current_entries, 0);
    assert_eq!(zero.cache_metrics().oversize_bypasses, 2);
    assert_eq!(zero.cache_stats().hits, 0);

    let mut tiny = TerminalShaper::with_cache_budget(FontConfig::new("Noto Sans"), 1);
    let long = "long terminal row ".repeat(1_024);
    tiny.shape_row(&mut catalog, &long).expect("long shape");
    tiny.shape_row(&mut catalog, &long)
        .expect("long shape again");
    assert_eq!(tiny.cache_metrics().current_bytes, 0);
    assert_eq!(tiny.cache_metrics().oversize_bypasses, 2);
    assert_eq!(tiny.cache_stats().hits, 0);
}

#[test]
fn shape_cache_key_covers_catalog_identity_configuration_and_spans() {
    let mut first_catalog = catalog(&[LATIN]);
    let mut second_catalog = catalog(&[LATIN]);
    let mut shaper = TerminalShaper::with_cache_budget(FontConfig::new("Noto Sans"), 1 << 20);

    shaper
        .shape_row(&mut first_catalog, "fi")
        .expect("first catalog");
    shaper
        .shape_row(&mut second_catalog, "fi")
        .expect("second catalog");
    shaper.set_config(FontConfig::new("Noto Sans").with_ligatures(false));
    shaper
        .shape_row(&mut second_catalog, "fi")
        .expect("new config");

    assert_eq!(shaper.cache_stats().hits, 0);
    assert_eq!(shaper.cache_stats().misses, 3);
}

fn raster_request(row: &ShapedRow, glyph: &ShapedGlyph) -> RasterRequest {
    RasterRequest::for_shaped_glyph(row, glyph, glyph.x, glyph.y)
}

#[test]
fn raster_cache_returns_headless_masks_and_color_pixels() {
    let mut catalog = catalog(&[LATIN, EMOJI]);
    let config = FontConfig::new("Noto Sans").with_fallbacks(["Noto Color Emoji"]);
    let mut shaper = TerminalShaper::new(config);
    let latin = shaper.shape_row(&mut catalog, "A").expect("latin");
    let emoji = shaper.shape_row(&mut catalog, "😀").expect("emoji");
    let mut cache = RasterCache::new(RasterCacheConfig::new(1 << 20));

    let mask = cache
        .rasterize(&mut catalog, raster_request(&latin, &latin.glyphs[0]))
        .expect("latin raster");
    assert!(mask.width > 0 && mask.height > 0);
    assert!(matches!(mask.content, RasterContent::Mask(_)));
    assert_eq!(
        mask.content.bytes().len(),
        mask.width as usize * mask.height as usize
    );

    let color = cache
        .rasterize(&mut catalog, raster_request(&emoji, &emoji.glyphs[0]))
        .expect("emoji raster");
    assert!(matches!(color.content, RasterContent::Rgba(_)));
    assert_eq!(
        color.content.bytes().len(),
        color.width as usize * color.height as usize * 4
    );
    assert!(
        color
            .content
            .bytes()
            .chunks_exact(4)
            .any(|pixel| pixel[3] != 0)
    );
    assert!(
        color
            .content
            .bytes()
            .chunks_exact(4)
            .any(|pixel| pixel[0] != pixel[1] || pixel[1] != pixel[2])
    );
    let repeated = cache
        .rasterize(&mut catalog, raster_request(&emoji, &emoji.glyphs[0]))
        .expect("repeat emoji");
    assert_eq!(color, repeated);
}

#[test]
fn positioned_raster_preserves_integer_origins_without_duplicating_cached_pixels() {
    let mut catalog = catalog(&[LATIN]);
    let mut shaper = TerminalShaper::new(FontConfig::new("Noto Sans"));
    let row = shaper.shape_row(&mut catalog, "A").expect("shape");
    let glyph = &row.glyphs[0];
    let mut cache = RasterCache::new(RasterCacheConfig::new(1 << 20));

    let first = cache
        .rasterize_positioned(
            &mut catalog,
            RasterRequest::for_shaped_glyph(&row, glyph, 10.25, 20.75),
        )
        .expect("first positioned raster");
    let moved = cache
        .rasterize_positioned(
            &mut catalog,
            RasterRequest::for_shaped_glyph(&row, glyph, 42.25, 53.75),
        )
        .expect("moved positioned raster");

    assert_eq!((first.origin_x, first.origin_y), (10, 20));
    assert_eq!((moved.origin_x, moved.origin_y), (42, 53));
    assert!(
        std::sync::Arc::ptr_eq(&first.image, &moved.image),
        "integer translation must reuse one cached glyph bitmap"
    );

    cache.set_scale(2.0, 1.0);
    let physical = cache
        .rasterize_positioned(
            &mut catalog,
            RasterRequest::for_shaped_glyph_at_physical_position(&row, glyph, 10.25, 20.75),
        )
        .expect("physical-position raster");
    assert_eq!(
        (physical.origin_x, physical.origin_y),
        (10, 20),
        "an already physical framebuffer origin must not be scaled twice"
    );
}

#[test]
fn raster_cache_key_covers_scale_subpixel_flags_and_strong_font_scope() {
    let mut catalog = catalog(&[LATIN]);
    let mut shaper = TerminalShaper::new(FontConfig::new("Noto Sans"));
    let row = shaper.shape_row(&mut catalog, "A").expect("shape");
    let base = raster_request(&row, &row.glyphs[0]);
    let mut cache = RasterCache::new(RasterCacheConfig::new(1 << 20));

    cache.rasterize(&mut catalog, base).expect("base miss");
    cache.rasterize(&mut catalog, base).expect("base hit");
    cache
        .rasterize(
            &mut catalog,
            RasterRequest::for_shaped_glyph(&row, &row.glyphs[0], 0.5, row.glyphs[0].y),
        )
        .expect("subpixel miss");
    cache
        .rasterize(
            &mut catalog,
            RasterRequest::for_shaped_glyph(&row, &row.glyphs[0], 10.5, row.glyphs[0].y),
        )
        .expect("integral position shares bitmap");
    cache.set_scale(2.0, 1.0);
    cache.rasterize(&mut catalog, base).expect("dpi miss");
    cache.set_scale(2.0, 1.25);
    cache.rasterize(&mut catalog, base).expect("zoom miss");
    let mut flagged = row.glyphs[0].clone();
    flagged.raster_flags |= RasterFlags::DISABLE_HINTING;
    cache
        .rasterize(&mut catalog, raster_request(&row, &flagged))
        .expect("flags miss");

    assert_eq!(cache.metrics().hits, 2);
    assert_eq!(cache.metrics().misses, 5);
}

#[test]
fn raster_cache_eviction_zero_budget_oversize_and_stale_ids_are_safe() {
    let mut catalog = catalog(&[LATIN]);
    let mut shaper = TerminalShaper::new(FontConfig::new("Noto Sans"));
    let row = shaper.shape_row(&mut catalog, "ABC").expect("shape");

    let mut zero = RasterCache::new(RasterCacheConfig::new(0));
    let request = raster_request(&row, &row.glyphs[0]);
    assert!(zero.rasterize(&mut catalog, request).is_some());
    assert!(zero.rasterize(&mut catalog, request).is_some());
    assert_eq!(zero.metrics().current_entries, 0);
    assert_eq!(zero.metrics().oversize_bypasses, 2);

    let mut bounded = RasterCache::new(RasterCacheConfig::new(1_024));
    for glyph in &row.glyphs {
        bounded.set_scale(4.0, 1.0);
        let request = raster_request(&row, glyph);
        let _ = bounded.rasterize(&mut catalog, request);
    }
    let metrics = bounded.metrics();
    assert!(metrics.current_bytes <= 1_024);
    assert!(metrics.peak_bytes <= 1_024);
    assert!(metrics.evictions > 0 || metrics.oversize_bypasses > 0);

    let stale = request;
    catalog
        .load_source(source(EMOJI))
        .expect("advance generation");
    assert!(bounded.rasterize(&mut catalog, stale).is_none());
    assert_eq!(bounded.metrics().current_entries, 0);
    assert!(bounded.metrics().invalidations > 0);
}

#[test]
fn changing_raster_scale_invalidates_entries_and_invalid_values_never_panic() {
    let mut catalog = catalog(&[LATIN]);
    let mut shaper = TerminalShaper::new(FontConfig::new("Noto Sans"));
    let row = shaper.shape_row(&mut catalog, "A").expect("shape");
    let request = raster_request(&row, &row.glyphs[0]);
    let mut cache = RasterCache::new(RasterCacheConfig::new(1 << 20));

    cache.rasterize(&mut catalog, request).expect("first");
    cache.set_scale(2.0, 1.5);
    assert_eq!(cache.metrics().current_entries, 0);
    cache.rasterize(&mut catalog, request).expect("scaled");

    cache.set_scale(f32::INFINITY, 1.0);
    assert!(cache.rasterize(&mut catalog, request).is_none());
    cache.set_scale(1.0, -1.0);
    assert!(cache.rasterize(&mut catalog, request).is_none());
}

#[test]
fn raster_positions_are_rejected_before_subpixel_quantization_can_overflow() {
    let mut catalog = catalog(&[LATIN]);
    let mut shaper = TerminalShaper::new(FontConfig::new("Noto Sans"));
    let row = shaper.shape_row(&mut catalog, "A").expect("shape");
    let glyph = &row.glyphs[0];
    let mut cache = RasterCache::new(RasterCacheConfig::new(1 << 20));
    let positive_boundary = 2_147_483_648.0_f32;
    let negative_boundary = -2_147_483_648.0_f32;
    let positive_safe = f32::from_bits(positive_boundary.to_bits() - 1);
    let negative_safe = f32::from_bits(negative_boundary.to_bits() - 1);

    for coordinate in [
        f32::MAX,
        -f32::MAX,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        positive_boundary,
        f32::from_bits(positive_boundary.to_bits() + 1),
        negative_boundary,
        f32::from_bits(negative_boundary.to_bits() + 1),
    ] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cache.rasterize(
                &mut catalog,
                RasterRequest::for_shaped_glyph(&row, glyph, coordinate, coordinate),
            )
        }));
        assert!(result.is_ok(), "coordinate {coordinate:?} panicked");
        assert!(
            result.expect("checked above").is_none(),
            "coordinate {coordinate:?} was accepted"
        );
    }

    for coordinate in [
        positive_safe,
        negative_safe,
        2_000_000_000.0,
        -2_000_000_000.0,
    ] {
        assert!(
            cache
                .rasterize(
                    &mut catalog,
                    RasterRequest::for_shaped_glyph(&row, glyph, coordinate, coordinate),
                )
                .is_some(),
            "safe coordinate {coordinate:?} was rejected"
        );
    }

    let mut dangerous_offset = glyph.clone();
    dangerous_offset.x_offset = f32::MAX;
    dangerous_offset.y_offset = -f32::MAX;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cache.rasterize(
            &mut catalog,
            RasterRequest::for_shaped_glyph(&row, &dangerous_offset, 0.0, 0.0),
        )
    }));
    assert!(result.is_ok());
    assert!(result.expect("checked above").is_none());
}

#[test]
fn blank_glyphs_are_negative_cached_without_hiding_visible_fallbacks() {
    let mut catalog = catalog(&[LATIN]);
    let mut shaper = TerminalShaper::new(FontConfig::new("Noto Sans"));
    let tofu = shaper.shape_row(&mut catalog, "Ω").expect("tofu row");
    assert!(tofu.glyphs[0].is_tofu);
    let blank = shaper.shape_row(&mut catalog, " ").expect("blank row");
    let mut cache = RasterCache::new(RasterCacheConfig::new(64 * 1024));

    let fallback = cache
        .rasterize(&mut catalog, raster_request(&tofu, &tofu.glyphs[0]))
        .expect("visible tofu fallback");
    assert_eq!(fallback.fallback, Some(RasterFallback::MissingGlyph));
    assert!(matches!(fallback.content, RasterContent::Mask(_)));
    assert!(cache.metrics().current_bytes <= cache.metrics().budget_bytes);

    let blank_glyph = blank.glyphs.first().expect("blank row glyph");
    let blank_request = raster_request(&blank, blank_glyph);
    assert!(cache.rasterize(&mut catalog, blank_request).is_none());
    let after_blank_miss = cache.metrics();
    assert_eq!(after_blank_miss.current_entries, 2);
    assert!(after_blank_miss.current_bytes > 0);

    assert!(cache.rasterize(&mut catalog, blank_request).is_none());
    let after_blank_hit = cache.metrics();
    assert_eq!(after_blank_hit.hits, after_blank_miss.hits + 1);
    assert_eq!(after_blank_hit.misses, after_blank_miss.misses);

    let mut visible = blank.clone();
    visible.text = "A".to_owned();
    let visible_request = raster_request(&visible, &visible.glyphs[0]);
    let visible_fallback = cache
        .rasterize(&mut catalog, visible_request)
        .expect("visible request must retain a tofu fallback");
    assert_eq!(
        visible_fallback.fallback,
        Some(RasterFallback::RasterFailure)
    );
    assert_eq!(cache.metrics().raster_failures, 1);
}

#[test]
fn negative_raster_entries_follow_scale_catalog_and_budget_scopes() {
    let mut catalog = catalog(&[LATIN]);
    let mut shaper = TerminalShaper::new(FontConfig::new("Noto Sans"));
    let blank = shaper.shape_row(&mut catalog, " ").expect("blank row");
    let blank_glyph = blank.glyphs.first().expect("blank row glyph");
    let request = raster_request(&blank, blank_glyph);
    let mut cache = RasterCache::new(RasterCacheConfig::new(64 * 1024));

    assert!(cache.rasterize(&mut catalog, request).is_none());
    assert!(cache.rasterize(&mut catalog, request).is_none());
    let first = cache.metrics();
    assert_eq!(first.current_entries, 1);
    assert!(
        first.current_bytes > 0,
        "negative result must charge metadata"
    );

    let mut flagged = blank_glyph.clone();
    flagged.raster_flags |= RasterFlags::DISABLE_HINTING;
    let flagged_request = raster_request(&blank, &flagged);
    cache.set_budget(first.current_bytes);
    assert!(cache.rasterize(&mut catalog, flagged_request).is_none());
    let after_eviction = cache.metrics();
    assert!(after_eviction.evictions > 0);
    assert!(after_eviction.current_bytes <= after_eviction.budget_bytes);
    assert!(after_eviction.current_entries <= 1);

    let invalidations_before_catalog = cache.metrics().invalidations;
    catalog
        .load_source(source(CJK))
        .expect("catalog generation update");
    let refreshed = shaper
        .shape_row(&mut catalog, " ")
        .expect("blank row after catalog update");
    assert!(
        cache
            .rasterize(
                &mut catalog,
                raster_request(&refreshed, &refreshed.glyphs[0])
            )
            .is_none()
    );
    assert!(cache.metrics().invalidations > invalidations_before_catalog);

    let invalidations_before_scale = cache.metrics().invalidations;
    cache.set_scale(2.0, 1.0);
    assert!(cache.metrics().invalidations > invalidations_before_scale);
    assert!(
        cache
            .rasterize(
                &mut catalog,
                raster_request(&refreshed, &refreshed.glyphs[0])
            )
            .is_none()
    );
}

#[test]
fn corrupt_visible_color_bitmap_falls_back_without_panicking() {
    let mut bytes = std::fs::read(fixture_path(EMOJI)).expect("read emoji");
    let idat_offsets: Vec<_> = bytes
        .windows(4)
        .enumerate()
        .filter_map(|(offset, chunk)| (chunk == b"IDAT").then_some(offset + 4))
        .collect();
    assert!(!idat_offsets.is_empty(), "embedded PNG IDAT");
    for offset in idat_offsets {
        bytes[offset] ^= 0xff;
    }
    let mut catalog = FontCatalog::from_sources("en-US", [FontSource::new("corrupt emoji", bytes)])
        .expect("sfnt remains parseable");
    let mut shaper = TerminalShaper::new(FontConfig::new("Noto Color Emoji"));
    let row = shaper.shape_row(&mut catalog, "😀").expect("shape emoji");
    let mut cache = RasterCache::new(RasterCacheConfig::new(64 * 1024));

    let fallback = cache
        .rasterize(&mut catalog, raster_request(&row, &row.glyphs[0]))
        .expect("visible raster fallback");
    assert_eq!(fallback.fallback, Some(RasterFallback::RasterFailure));
    assert!(matches!(fallback.content, RasterContent::Mask(_)));
    assert_eq!(cache.metrics().raster_failures, 1);
}

#[test]
fn headless_multiscript_specimen_rasterizes_without_system_fonts() {
    let mut catalog = catalog(&[LATIN, CJK, ARABIC, DEVANAGARI, EMOJI]);
    let config = FontConfig::new("Noto Sans").with_fallbacks([
        "Noto Sans SC",
        "Noto Sans Arabic",
        "Noto Sans Devanagari",
        "Noto Color Emoji",
    ]);
    let mut shaper = TerminalShaper::new(config);
    let mut cache = RasterCache::new(RasterCacheConfig::new(2 * 1024 * 1024));

    for specimen in ["A", "中", "مرحبا", "नमस्ते", "😀"] {
        let row = shaper
            .shape_row(&mut catalog, specimen)
            .expect("shape specimen");
        assert!(!row.glyphs.is_empty(), "{specimen:?} shaped no glyphs");
        assert!(
            row.glyphs.iter().any(|glyph| {
                cache
                    .rasterize(&mut catalog, raster_request(&row, glyph))
                    .is_some()
            }),
            "{specimen:?} produced no raster"
        );
    }
    assert!(cache.metrics().current_bytes <= cache.metrics().budget_bytes);
}
