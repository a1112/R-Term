use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use font8x8::UnicodeFonts as _;
use rssh_terminal::{Cell, Color, CursorShape, InlineImageFormat, Terminal, TerminalGrid};
use rterm_render_core::render_inline_images_from_terminal;
use rterm_types::TerminalSize;

use super::{
    BASIC_FONTS, DamageRegion, DecodedImage, ImageDrawPlan, ImageTiePolicy, PixelRenderer, Rect,
    RenderBackgroundGradientHsb, RenderBackgroundImage, RenderBackgroundImageAttachment,
    RenderBackgroundImageDimension, RenderBackgroundImageHorizontalAlign,
    RenderBackgroundImageLength, RenderBackgroundImageRepeat, RenderBackgroundImageVerticalAlign,
    RenderBoldBrightensAnsiColors, RenderCell, RenderGeometry, RenderInlineImage,
    RenderInlineImageFragment, SCROLLBAR_THUMB_COLOR, SCROLLBAR_TRACK_COLOR, ScrollbackScrollbar,
    Surface, TerminalRenderSnapshot, background_image_axis_coordinate, background_image_layout,
    build_image_draw_plan, compare_image_draw_plans, for_each_image_draw_span,
    for_each_opaque_glyph_row_run, terminal_first_row_pixel_digest,
};

fn ordering_plan(
    kitty_image_id: Option<u32>,
    stable_order: usize,
    tie_policy: ImageTiePolicy,
    z_index: i32,
) -> ImageDrawPlan {
    ImageDrawPlan {
        destination_x: 0,
        destination_y: 0,
        width: 1,
        height: 1,
        decoded: Arc::new(DecodedImage {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, u8::MAX].into(),
        }),
        sample_source_x: 0,
        sample_source_y: 0,
        sample_target_x: 0,
        sample_target_y: 0,
        sample_source_width: 1,
        sample_source_height: 1,
        sample_destination_width: 1,
        sample_destination_height: 1,
        z_index,
        kitty_image_id,
        parent_index: stable_order,
        fragment_index: stable_order,
        tie_policy,
        stable_order,
    }
}

fn legacy_surface_fill(target: &mut [u8], width: u32, height: u32, color: [u8; 4]) {
    let pixel_count =
        usize::try_from(u64::from(width).saturating_mul(u64::from(height))).unwrap_or(usize::MAX);
    for pixel in target.chunks_exact_mut(4).take(pixel_count) {
        pixel.copy_from_slice(&color);
    }
}

fn legacy_surface_fill_rect(
    target: &mut [u8],
    surface_size: (u32, u32),
    rect: Rect,
    color: [u8; 4],
    alpha: Option<u8>,
) {
    if matches!(alpha, Some(0)) {
        return;
    }
    let (width, height) = surface_size;
    let max_y = rect.y.saturating_add(rect.height).min(height);
    let max_x = rect.x.saturating_add(rect.width).min(width);
    let alpha = alpha.map(u16::from);
    let inverse_alpha = alpha.map(|value| u16::from(u8::MAX).saturating_sub(value));
    for row in rect.y..max_y {
        for column in rect.x..max_x {
            let index =
                usize::try_from((u64::from(row) * u64::from(width) + u64::from(column)) * 4)
                    .unwrap_or(usize::MAX);
            if let Some(pixel) = target.get_mut(index..index.saturating_add(4)) {
                if let (Some(alpha), Some(inverse_alpha)) = (alpha, inverse_alpha)
                    && alpha != u16::from(u8::MAX)
                {
                    pixel[0] = super::blend_channel(color[0], pixel[0], alpha, inverse_alpha);
                    pixel[1] = super::blend_channel(color[1], pixel[1], alpha, inverse_alpha);
                    pixel[2] = super::blend_channel(color[2], pixel[2], alpha, inverse_alpha);
                    pixel[3] = u8::MAX;
                } else {
                    pixel.copy_from_slice(&color);
                }
            }
        }
    }
}

fn legacy_basic_glyph_8x16(
    surface: &mut Surface<'_>,
    glyph: [u8; 8],
    origin_x: u32,
    origin_y: u32,
    color: [u8; 4],
) {
    for (glyph_y, row_bits) in glyph.iter().enumerate() {
        for glyph_x in 0..8 {
            if row_bits & (1 << glyph_x) == 0 {
                continue;
            }
            surface.fill_rect(
                Rect {
                    x: origin_x + glyph_x,
                    y: origin_y + u32::try_from(glyph_y).unwrap() * 2,
                    width: 1,
                    height: 2,
                },
                color,
            );
        }
    }
}

#[test]
fn surface_fill_matches_complete_pixel_reference_for_short_targets() {
    let color = [17, 33, 65, 129];
    for target_len in [0, 1, 3, 4, 5, 15, 16, 17, 47, 48, 49, 64] {
        let initial = (0..target_len)
            .map(|index| u8::try_from(index % 251).unwrap())
            .collect::<Vec<_>>();
        let mut expected = initial.clone();
        legacy_surface_fill(&mut expected, 4, 3, color);
        let mut actual = initial;

        Surface {
            target: &mut actual,
            width: 4,
            height: 3,
        }
        .fill(color);

        assert_eq!(actual, expected, "target_len={target_len}");
    }
}

#[test]
fn surface_rect_fills_match_clipped_reference_for_every_alpha_edge() {
    let color = [231, 117, 9, 77];
    let rects = [
        Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        },
        Rect {
            x: 1,
            y: 1,
            width: u32::MAX,
            height: u32::MAX,
        },
        Rect {
            x: 4,
            y: 0,
            width: 1,
            height: 3,
        },
        Rect {
            x: 0,
            y: 3,
            width: 4,
            height: 1,
        },
        Rect {
            x: u32::MAX,
            y: u32::MAX,
            width: u32::MAX,
            height: u32::MAX,
        },
        Rect {
            x: 1,
            y: 1,
            width: 0,
            height: 2,
        },
        Rect {
            x: 1,
            y: 1,
            width: 2,
            height: 0,
        },
    ];
    for target_len in [0, 1, 3, 4, 17, 31, 47, 48, 53] {
        for rect in rects {
            for alpha in [0, 1, 128, u8::MAX] {
                let initial = (0..target_len)
                    .map(|index| u8::try_from((index * 17 + 3) % 251).unwrap())
                    .collect::<Vec<_>>();
                let mut expected = initial.clone();
                legacy_surface_fill_rect(&mut expected, (4, 3), rect, color, Some(alpha));
                let mut actual = initial;

                Surface {
                    target: &mut actual,
                    width: 4,
                    height: 3,
                }
                .fill_rect_alpha(rect, color, alpha);

                assert_eq!(
                    actual, expected,
                    "target_len={target_len} rect=({}, {}, {}, {}) alpha={alpha}",
                    rect.x, rect.y, rect.width, rect.height
                );
            }

            let initial = (0..target_len)
                .map(|index| u8::try_from((index * 17 + 3) % 251).unwrap())
                .collect::<Vec<_>>();
            let mut expected = initial.clone();
            legacy_surface_fill_rect(&mut expected, (4, 3), rect, color, None);
            let mut actual = initial;
            Surface {
                target: &mut actual,
                width: 4,
                height: 3,
            }
            .fill_rect(rect, color);
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn surface_exposes_one_complete_rgba_span_per_clipped_row() {
    let mut target = vec![0; 31];
    let surface = Surface {
        target: &mut target,
        width: 4,
        height: 3,
    };

    assert_eq!(surface.clipped_row_byte_range(0, 1, 4), Some(4..16));
    assert_eq!(surface.clipped_row_byte_range(1, 1, 4), Some(20..28));
    assert_eq!(surface.clipped_row_byte_range(2, 1, 4), None);
    assert_eq!(surface.clipped_row_byte_range(3, 0, 4), None);
    assert_eq!(surface.clipped_row_byte_range(0, 4, 4), None);
}

#[test]
fn opaque_glyph_runs_match_every_legacy_row_mask_and_reduce_rect_calls() {
    let color = [231, 117, 9, u8::MAX];
    for row_bits in 0..=u8::MAX {
        for bold in [false, true] {
            for cell_width in [8, 10, 16] {
                let scale_x = cell_width.max(8) / 8;
                for row_offset in [0, 1, 2] {
                    let mut expected = vec![0; 20 * 2 * 4];
                    let mut expected_surface = Surface {
                        target: &mut expected,
                        width: 20,
                        height: 2,
                    };
                    for glyph_x in 0..8 {
                        if row_bits & (1 << glyph_x) == 0 {
                            continue;
                        }
                        let draw_x = glyph_x * scale_x + row_offset;
                        if let Some(width) =
                            super::clipped_cell_width(draw_x, 0, cell_width, scale_x)
                        {
                            expected_surface.fill_rect(
                                Rect {
                                    x: draw_x,
                                    y: 0,
                                    width,
                                    height: 2,
                                },
                                color,
                            );
                        }
                        let bold_x = draw_x.saturating_add(scale_x);
                        if bold && bold_x < cell_width {
                            expected_surface.fill_rect(
                                Rect {
                                    x: bold_x,
                                    y: 0,
                                    width: scale_x,
                                    height: 2,
                                },
                                color,
                            );
                        }
                    }

                    let mut actual = vec![0; 20 * 2 * 4];
                    let mut actual_surface = Surface {
                        target: &mut actual,
                        width: 20,
                        height: 2,
                    };
                    for_each_opaque_glyph_row_run(
                        row_bits,
                        0,
                        cell_width,
                        scale_x,
                        row_offset,
                        bold,
                        |x, width| {
                            actual_surface.fill_rect(
                                Rect {
                                    x,
                                    y: 0,
                                    width,
                                    height: 2,
                                },
                                color,
                            );
                        },
                    );

                    assert_eq!(
                        actual, expected,
                        "bits={row_bits:08b} bold={bold} width={cell_width} offset={row_offset}"
                    );
                }
            }
        }
    }

    let mut calls = 0;
    for_each_opaque_glyph_row_run(u8::MAX, 0, 8, 1, 0, true, |_, _| calls += 1);
    assert_eq!(calls, 1, "one dense glyph row should be one opaque fill");
}

#[test]
fn basic_opaque_glyph_fast_path_matches_generic_pixels_and_rejects_partial_targets() {
    let color = [231, 117, 9, u8::MAX];
    let mut glyphs = (0..=u8::MAX).map(|mask| [mask; 8]).collect::<Vec<_>>();
    glyphs.extend(
        (0_u32..=127)
            .filter_map(char::from_u32)
            .filter_map(|ch| BASIC_FONTS.get(ch)),
    );

    for glyph in &glyphs {
        for (width, height, origin_x, origin_y) in [(24, 20, 3, 2), (16, 16, 8, 0), (8, 16, 0, 0)] {
            let mut expected = vec![0; width * height * 4];
            legacy_basic_glyph_8x16(
                &mut Surface {
                    target: &mut expected,
                    width: u32::try_from(width).unwrap(),
                    height: u32::try_from(height).unwrap(),
                },
                *glyph,
                u32::try_from(origin_x).unwrap(),
                u32::try_from(origin_y).unwrap(),
                color,
            );
            let mut actual = vec![0; width * height * 4];
            let rendered = Surface {
                target: &mut actual,
                width: u32::try_from(width).unwrap(),
                height: u32::try_from(height).unwrap(),
            }
            .try_fill_basic_glyph_8x16(
                *glyph,
                u32::try_from(origin_x).unwrap(),
                u32::try_from(origin_y).unwrap(),
                color,
            );
            assert!(rendered);
            assert_eq!(
                actual, expected,
                "glyph={glyph:?} origin=({origin_x},{origin_y})"
            );
        }
    }

    for (target_len, origin_x, origin_y) in [
        (8 * 16 * 4 - 1, 0, 0),
        (8 * 16 * 4, 1, 0),
        (8 * 16 * 4, 0, 1),
    ] {
        let mut target = vec![17; target_len];
        let before = target.clone();
        let rendered = Surface {
            target: &mut target,
            width: 8,
            height: 16,
        }
        .try_fill_basic_glyph_8x16([u8::MAX; 8], origin_x, origin_y, color);
        assert!(!rendered);
        assert_eq!(target, before);
    }
}

#[test]
fn image_draw_order_is_transitive_and_groups_whole_images_before_fragments() {
    let id_100 = ordering_plan(Some(100), 0, ImageTiePolicy::Whole, 0);
    let missing = ordering_plan(None, 1, ImageTiePolicy::Whole, 0);
    let id_1 = ordering_plan(Some(1), 2, ImageTiePolicy::Whole, 0);
    let plans = [&id_100, &missing, &id_1];
    for left in plans {
        for right in plans {
            assert_eq!(
                compare_image_draw_plans(left, right),
                compare_image_draw_plans(right, left).reverse()
            );
            for third in plans {
                if compare_image_draw_plans(left, right).is_le()
                    && compare_image_draw_plans(right, third).is_le()
                {
                    assert!(compare_image_draw_plans(left, third).is_le());
                }
            }
        }
    }
    assert!(compare_image_draw_plans(&id_1, &id_100).is_lt());
    assert!(compare_image_draw_plans(&id_100, &missing).is_lt());
    assert!(compare_image_draw_plans(&id_1, &missing).is_lt());
    for mut input in [
        vec![missing.clone(), id_100.clone(), id_1.clone()],
        vec![id_1.clone(), id_100.clone(), missing.clone()],
    ] {
        input.sort_by(compare_image_draw_plans);
        assert_eq!(
            input
                .iter()
                .map(|draw| draw.kitty_image_id)
                .collect::<Vec<_>>(),
            vec![Some(1), Some(100), None]
        );
    }

    let whole = ordering_plan(None, 9, ImageTiePolicy::Whole, 10);
    let fragment = ordering_plan(Some(1), 0, ImageTiePolicy::Fragment, 0);
    assert!(
        compare_image_draw_plans(&whole, &fragment).is_lt(),
        "whole image draws must remain a stable group before fragments"
    );

    let earlier_missing = ordering_plan(None, 3, ImageTiePolicy::Whole, 0);
    let later_missing = ordering_plan(None, 4, ImageTiePolicy::Whole, 0);
    assert!(compare_image_draw_plans(&earlier_missing, &later_missing).is_lt());

    let lower_z_whole = ordering_plan(None, 5, ImageTiePolicy::Whole, 1);
    assert!(compare_image_draw_plans(&lower_z_whole, &whole).is_lt());
    let ultra_fragment = ordering_plan(None, 6, ImageTiePolicy::Fragment, i32::MIN / 2 - 1);
    assert!(
        compare_image_draw_plans(&ultra_fragment, &lower_z_whole).is_lt(),
        "layer ordering must precede the whole/fragment grouping"
    );
}

#[test]
fn one_pixel_damage_samples_only_one_pixel_of_a_large_image_draw() {
    let draw_rect = Rect {
        x: 0,
        y: 0,
        width: 3_000,
        height: 3_000,
    };
    let mut full_pixels = 0_u64;
    for_each_image_draw_span(draw_rect, None, |_, start_x, end_x| {
        full_pixels += u64::from(end_x - start_x);
    });
    let mut damaged_pixels = 0_u64;
    for_each_image_draw_span(
        draw_rect,
        Some(&[Rect {
            x: 1_500,
            y: 1_500,
            width: 1,
            height: 1,
        }]),
        |_, start_x, end_x| damaged_pixels += u64::from(end_x - start_x),
    );

    assert_eq!(full_pixels, 9_000_000);
    assert_eq!(damaged_pixels, 1);
}

#[test]
fn overlapping_damage_samples_each_covered_pixel_once() {
    let mut sampled_pixels = 0_u64;
    let mut span_visits = 0;
    let damage = [
        Rect {
            x: 10,
            y: 10,
            width: 4,
            height: 2,
        },
        Rect {
            x: 12,
            y: 10,
            width: 4,
            height: 2,
        },
        Rect {
            x: 10,
            y: 10,
            width: 4,
            height: 2,
        },
    ];

    for_each_image_draw_span(
        Rect {
            x: 0,
            y: 0,
            width: 3_000,
            height: 3_000,
        },
        Some(&damage),
        |_, start_x, end_x| {
            span_visits += 1;
            sampled_pixels += u64::from(end_x - start_x);
        },
    );

    assert_eq!(span_visits, 2);
    assert_eq!(sampled_pixels, 12);
}

#[test]
fn fragmented_parent_is_decoded_once_and_shares_one_source_allocation() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=77,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let (draws, metrics) =
        build_image_draw_plan(&snapshot, RenderGeometry::new(2, 2, 1, 1), 0, None, None);

    assert_eq!(draws.len(), 4);
    assert_eq!(metrics.decode_count, 1);
    assert_eq!(metrics.unique_decoded_bytes, 16);
    assert!(
        draws
            .windows(2)
            .all(|pair| Arc::ptr_eq(&pair[0].decoded, &pair[1].decoded))
    );
}

#[test]
fn lightweight_planner_keeps_two_draws_whose_scaled_pixels_exceed_64_mib() {
    const RED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(
        format!("\x1b]1337;File=inline=1;width=3000px;height=3000px:{RED_PNG}\x07").as_bytes(),
    );
    terminal.feed(b"\x1b[H");
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=1,f=24,s=1,v=1,c=1,r=1;AAD/\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let (draws, metrics) = build_image_draw_plan(
        &snapshot,
        RenderGeometry::new(3_000, 3_000, 3_000, 3_000),
        0,
        None,
        None,
    );

    assert_eq!(draws.len(), 2);
    assert!(
        draws
            .iter()
            .map(|draw| u64::from(draw.width) * u64::from(draw.height) * 4)
            .sum::<u64>()
            > 64 * 1024 * 1024
    );
    assert!(metrics.unique_decoded_bytes < 1024);
    assert_eq!(
        super::image_draw_pixel(&draws[1], 2_999, 2_999),
        [0, 0, 255, 255]
    );
}

#[test]
fn zero_width_region_is_empty() {
    assert!(DamageRegion::new(0, 0, 0, 1).is_empty());
}

#[test]
fn background_image_layout_resolves_percent_cell_offsets_and_repeat_sizes() {
    let image = RenderBackgroundImage {
        data: Vec::new(),
        opacity_alpha: u8::MAX,
        hsb: RenderBackgroundGradientHsb::IDENTITY,
        animation_speed_millis: 1_000,
        attachment: RenderBackgroundImageAttachment::Fixed,
        width: RenderBackgroundImageDimension::Percent(5_000),
        height: RenderBackgroundImageDimension::Cells(2),
        repeat_x: RenderBackgroundImageRepeat::Repeat,
        repeat_y: RenderBackgroundImageRepeat::Repeat,
        horizontal_align: RenderBackgroundImageHorizontalAlign::Left,
        vertical_align: RenderBackgroundImageVerticalAlign::Top,
        horizontal_offset: RenderBackgroundImageLength::Cells(1),
        vertical_offset: RenderBackgroundImageLength::Percent(1_000),
        repeat_x_size: Some(RenderBackgroundImageLength::Percent(2_500)),
        repeat_y_size: Some(RenderBackgroundImageLength::Cells(3)),
    };
    let decoded = DecodedImage {
        width: 1,
        height: 1,
        pixels: Arc::from([]),
    };

    let layout = background_image_layout(&image, &decoded, 640, 400, 8, 16)
        .expect("expected background image layout");

    assert_eq!(layout.origin_x, 8);
    assert_eq!(layout.origin_y, 40);
    assert_eq!(layout.width, 320);
    assert_eq!(layout.height, 32);
    assert_eq!(layout.repeat_width, 160);
    assert_eq!(layout.repeat_height, 48);
}

#[test]
fn background_image_layout_resolves_contain_sizing() {
    let image = RenderBackgroundImage {
        data: Vec::new(),
        opacity_alpha: u8::MAX,
        hsb: RenderBackgroundGradientHsb::IDENTITY,
        animation_speed_millis: 1_000,
        attachment: RenderBackgroundImageAttachment::Fixed,
        width: RenderBackgroundImageDimension::Contain,
        height: RenderBackgroundImageDimension::Contain,
        repeat_x: RenderBackgroundImageRepeat::NoRepeat,
        repeat_y: RenderBackgroundImageRepeat::NoRepeat,
        horizontal_align: RenderBackgroundImageHorizontalAlign::Right,
        vertical_align: RenderBackgroundImageVerticalAlign::Bottom,
        horizontal_offset: RenderBackgroundImageLength::Pixels(0),
        vertical_offset: RenderBackgroundImageLength::Pixels(0),
        repeat_x_size: None,
        repeat_y_size: None,
    };
    let decoded = DecodedImage {
        width: 2,
        height: 1,
        pixels: Arc::from([]),
    };

    let layout = background_image_layout(&image, &decoded, 100, 80, 8, 16)
        .expect("expected background image layout");

    assert_eq!(layout.origin_x, 0);
    assert_eq!(layout.origin_y, 30);
    assert_eq!(layout.width, 100);
    assert_eq!(layout.height, 50);
}

#[test]
fn background_image_axis_coordinate_mirrors_alternate_tiles() {
    assert_eq!(
        background_image_axis_coordinate(0, 2, 2, RenderBackgroundImageRepeat::Mirror),
        Some(0)
    );
    assert_eq!(
        background_image_axis_coordinate(1, 2, 2, RenderBackgroundImageRepeat::Mirror),
        Some(1)
    );
    assert_eq!(
        background_image_axis_coordinate(2, 2, 2, RenderBackgroundImageRepeat::Mirror),
        Some(1)
    );
    assert_eq!(
        background_image_axis_coordinate(3, 2, 2, RenderBackgroundImageRepeat::Mirror),
        Some(0)
    );
}

#[test]
fn pixel_renderer_applies_background_image_animation_speed() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut renderer = PixelRenderer::with_animation_elapsed_ms(60);
    renderer.set_default_background_image(Some(RenderBackgroundImage {
        data: red_green_gif_bytes().to_vec(),
        opacity_alpha: u8::MAX,
        hsb: RenderBackgroundGradientHsb::IDENTITY,
        animation_speed_millis: 2_000,
        attachment: RenderBackgroundImageAttachment::Fixed,
        width: RenderBackgroundImageDimension::Cover,
        height: RenderBackgroundImageDimension::Cover,
        repeat_x: RenderBackgroundImageRepeat::Repeat,
        repeat_y: RenderBackgroundImageRepeat::Repeat,
        horizontal_align: RenderBackgroundImageHorizontalAlign::Left,
        vertical_align: RenderBackgroundImageVerticalAlign::Top,
        horizontal_offset: RenderBackgroundImageLength::Pixels(0),
        vertical_offset: RenderBackgroundImageLength::Pixels(0),
        repeat_x_size: None,
        repeat_y_size: None,
    }));
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [0, 255, 0, 255]);
}

#[test]
fn first_row_pixel_probe_is_stable_and_ignores_rows_below_the_region() {
    let mut terminal = Terminal::new(TerminalSize::new(16, 2));
    terminal.feed(b"fixture-ready\r\nsecond-row");
    let first = terminal_first_row_pixel_digest(&TerminalRenderSnapshot::from_terminal(&terminal));
    terminal.feed(b"X");
    let second = terminal_first_row_pixel_digest(&TerminalRenderSnapshot::from_terminal(&terminal));

    assert_eq!(first, second);
    assert_eq!(
        first,
        [
            62, 28, 153, 18, 24, 213, 227, 222, 223, 0, 225, 215, 88, 101, 29, 102, 97, 249, 72,
            90, 176, 163, 200, 110, 249, 197, 195, 88, 61, 22, 143, 100,
        ]
    );
}

#[test]
fn pixel_renderer_scrolls_background_image_attachment_with_viewport() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 2));
    terminal.feed(b"\x1b[?25l \r\n \r\n \r\n");
    let snapshot = TerminalRenderSnapshot::from_terminal_viewport(
        &terminal,
        terminal.scrollback().len().saturating_sub(1),
    );
    assert_eq!(snapshot.scrollback_offset(), 1);
    let mut renderer = PixelRenderer::default();
    renderer.set_default_background_image(Some(RenderBackgroundImage {
        data: red_green_blue_vertical_png_bytes().to_vec(),
        opacity_alpha: u8::MAX,
        hsb: RenderBackgroundGradientHsb::IDENTITY,
        animation_speed_millis: 1_000,
        attachment: RenderBackgroundImageAttachment::Scroll,
        width: RenderBackgroundImageDimension::Pixels(1),
        height: RenderBackgroundImageDimension::Pixels(3),
        repeat_x: RenderBackgroundImageRepeat::Repeat,
        repeat_y: RenderBackgroundImageRepeat::Repeat,
        horizontal_align: RenderBackgroundImageHorizontalAlign::Left,
        vertical_align: RenderBackgroundImageVerticalAlign::Top,
        horizontal_offset: RenderBackgroundImageLength::Pixels(0),
        vertical_offset: RenderBackgroundImageLength::Pixels(0),
        repeat_x_size: None,
        repeat_y_size: None,
    }));
    let mut target = vec![0; 4];

    renderer.render(&snapshot, &mut target, 1, 1, 1, 1);

    assert_eq!(pixel_at(&target, 1, 0, 0), [0, 255, 0, 255]);
}

#[test]
fn pixel_renderer_applies_background_image_parallax_factor() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 2));
    terminal.feed(b"\x1b[?25l \r\n \r\n \r\n \r\n");
    let snapshot = TerminalRenderSnapshot::from_terminal_viewport(&terminal, 2);
    assert_eq!(snapshot.scrollback_offset(), 2);
    let mut renderer = PixelRenderer::default();
    renderer.set_default_background_image(Some(RenderBackgroundImage {
        data: red_green_blue_vertical_png_bytes().to_vec(),
        opacity_alpha: u8::MAX,
        hsb: RenderBackgroundGradientHsb::IDENTITY,
        animation_speed_millis: 1_000,
        attachment: RenderBackgroundImageAttachment::Parallax { factor_millis: 500 },
        width: RenderBackgroundImageDimension::Pixels(1),
        height: RenderBackgroundImageDimension::Pixels(3),
        repeat_x: RenderBackgroundImageRepeat::Repeat,
        repeat_y: RenderBackgroundImageRepeat::Repeat,
        horizontal_align: RenderBackgroundImageHorizontalAlign::Left,
        vertical_align: RenderBackgroundImageVerticalAlign::Top,
        horizontal_offset: RenderBackgroundImageLength::Pixels(0),
        vertical_offset: RenderBackgroundImageLength::Pixels(0),
        repeat_x_size: None,
        repeat_y_size: None,
    }));
    let mut target = vec![0; 4];

    renderer.render(&snapshot, &mut target, 1, 1, 1, 1);

    assert_eq!(pixel_at(&target, 1, 0, 0), [0, 255, 0, 255]);
}

#[test]
fn render_snapshot_contains_non_blank_terminal_cells() {
    let mut grid = TerminalGrid::new(TerminalSize::new(3, 2));
    let mut cell = Cell::with_char('R');
    cell.foreground = Color::Indexed(2);
    cell.background = Color::Rgb(1, 2, 3);
    cell.bold = true;
    cell.underline = true;
    grid.set(1, 2, cell);

    let snapshot = TerminalRenderSnapshot::from_grid(&grid);

    assert_eq!(snapshot.cells().len(), 1);
    assert_eq!(snapshot.cells()[0].row, 1);
    assert_eq!(snapshot.cells()[0].column, 2);
    assert_eq!(snapshot.cells()[0].ch, 'R');
    assert_eq!(snapshot.cells()[0].foreground, Color::Indexed(2));
    assert_eq!(snapshot.cells()[0].background, Color::Rgb(1, 2, 3));
    assert!(snapshot.cells()[0].bold);
    assert!(snapshot.cells()[0].underline);
    assert!(!snapshot.cells()[0].inverse);
}

#[test]
fn render_snapshot_preserves_grapheme_once_on_leader() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 1));
    terminal.feed("👍🏽".as_bytes());

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let leader = snapshot
        .cells()
        .iter()
        .find(|cell| cell.column == 0)
        .unwrap();
    let continuation = snapshot
        .cells()
        .iter()
        .find(|cell| cell.column == 1)
        .unwrap();
    assert_eq!(leader.text.as_ref(), "👍🏽");
    assert_eq!(leader.columns, 2);
    assert!(!leader.continuation);
    assert_eq!(continuation.text.as_ref(), "");
    assert!(continuation.continuation);
    assert_eq!(continuation.ch, ' ');
}

#[test]
fn render_snapshot_reports_missing_glyph_codepoints_once() {
    let mut grid = TerminalGrid::new(TerminalSize::new(3, 1));
    for (column, ch) in [(0, 'R'), (1, '中'), (2, '中')] {
        grid.set(0, column, Cell::with_char(ch));
    }

    let snapshot = TerminalRenderSnapshot::from_grid(&grid);

    assert_eq!(snapshot.missing_glyphs(), vec!['中']);
}

#[test]
fn render_snapshot_treats_modern_ui_symbols_as_fallback_backed() {
    let mut grid = TerminalGrid::new(TerminalSize::new(4, 1));
    for (column, ch) in [(0, '×'), (1, '▾'), (2, '—'), (3, '□')] {
        grid.set(0, column, Cell::with_char(ch));
    }

    let snapshot = TerminalRenderSnapshot::from_grid(&grid);

    assert!(snapshot.missing_glyphs().is_empty());
}

#[test]
fn render_snapshot_preserves_inverse_style() {
    let mut grid = TerminalGrid::new(TerminalSize::new(1, 1));
    let mut cell = Cell::with_char('I');
    cell.inverse = true;
    grid.set(0, 0, cell);

    let snapshot = TerminalRenderSnapshot::from_grid(&grid);

    assert!(snapshot.cells()[0].inverse);
}

#[test]
fn render_snapshot_applies_screen_reverse_video_mode() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 1));

    terminal.feed(b"A\x1b[?5hB");
    let reversed = TerminalRenderSnapshot::from_terminal(&terminal);

    assert_eq!(snapshot_char(&reversed, 0, 0), Some('A'));
    assert_eq!(snapshot_char(&reversed, 0, 1), Some('B'));
    assert!(reversed.cells().iter().all(|cell| cell.inverse));
    assert_eq!(
        reversed.cells().len(),
        4,
        "reverse video should render the full visible screen"
    );

    terminal.feed(b"\x1b[?5lC");
    let normal = TerminalRenderSnapshot::from_terminal(&terminal);

    assert!(normal.cells().iter().all(|cell| !cell.inverse));
    assert!(normal.cells().len() < 4);
}

#[test]
fn render_snapshot_preserves_strikethrough_style() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[9mS");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert!(snapshot.cells()[0].strikethrough);
}

#[test]
fn render_snapshot_preserves_faint_style() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[2mF");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert!(snapshot.cells()[0].faint);
}

#[test]
fn render_snapshot_preserves_conceal_style() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[8mC");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert!(snapshot.cells()[0].conceal);
}

#[test]
fn render_snapshot_preserves_overline_style() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[53mO");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert!(snapshot.cells()[0].overline);
}

#[test]
fn render_snapshot_preserves_blink_style() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[5mB");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert!(snapshot.cells()[0].blink);
}

#[test]
fn render_snapshot_preserves_double_underline_style() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[21mD");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert!(snapshot.cells()[0].double_underline);
}

#[test]
fn render_snapshot_preserves_underline_color() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[4;58;2;1;2;3mU");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert_eq!(snapshot.cells()[0].underline_color, Color::Rgb(1, 2, 3));
}

#[test]
fn render_snapshot_preserves_colon_separated_underline_style() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[4:4mD");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert_eq!(
        snapshot.cells()[0].underline_style,
        rssh_terminal::UnderlineStyle::Dotted
    );
}

#[test]
fn render_snapshot_preserves_hyperlink_metadata() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 1));
    terminal.feed(b"\x1b]8;;https://example.com\x1b\\ab\x1b]8;;\x1b\\");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let grid_snapshot = TerminalRenderSnapshot::from_grid(terminal.grid());
    let cloned_snapshot = snapshot.clone();
    let source = terminal
        .grid()
        .get(0, 0)
        .unwrap()
        .hyperlink
        .as_ref()
        .unwrap();

    assert_eq!(
        snapshot.cells()[0].hyperlink.as_deref(),
        Some("https://example.com")
    );
    assert_eq!(
        snapshot.cells()[1].hyperlink.as_deref(),
        Some("https://example.com")
    );
    for rendered in [&snapshot, &grid_snapshot, &cloned_snapshot] {
        assert!(rendered.cells().iter().all(|cell| {
            cell.hyperlink
                .as_ref()
                .is_some_and(|hyperlink| hyperlink.as_ptr() == source.as_ptr())
        }));
    }
}

#[test]
fn render_snapshot_damage_keeps_terminal_hyperlink_storage_shared() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 1));
    terminal.feed(b"\x1b]8;;https://example.com/long/path\x1b\\abcd");
    terminal.take_damage();
    let mut snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    terminal.feed(b"\rZ");
    let damage = terminal.take_damage();
    snapshot.update_from_terminal_damage(&terminal, &damage);

    let source = terminal
        .grid()
        .get(0, 0)
        .unwrap()
        .hyperlink
        .as_ref()
        .unwrap();
    assert_eq!(snapshot.cells().len(), 4);
    assert!(snapshot.cells().iter().all(|cell| {
        cell.hyperlink
            .as_ref()
            .is_some_and(|hyperlink| hyperlink.as_ptr() == source.as_ptr())
    }));
}

#[test]
fn render_snapshot_preserves_iterm_inline_image_metadata() {
    let mut terminal = Terminal::new(TerminalSize::new(8, 2));
    terminal.feed(
            b"ab\x1b]1337;File=inline=1;name=aW1nLnBuZw==;size=4;width=3;height=2;preserveAspectRatio=0:QUJDRA==\x07cd",
        );

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert_eq!(
        snapshot.inline_images(),
        &[RenderInlineImage {
            row: 0,
            column: 2,
            name: Some("img.png".to_owned()),
            kitty_image_id: None,
            kitty_placement_id: None,
            kitty_z_index: None,
            size: Some(4),
            width: Some("3".to_owned()),
            height: Some("2".to_owned()),
            preserve_aspect_ratio: Some(false),
            image_format: InlineImageFormat::Encoded,
            pixel_width: None,
            pixel_height: None,
            source_x: None,
            source_y: None,
            source_width: None,
            source_height: None,
            target_x: None,
            target_y: None,
            data: b"ABCD".to_vec().into(),
        }]
    );
}

#[test]
fn graphics_fragment_snapshot_draws_cell_crops_without_parent_rectangle() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=77,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.inline_images().len(), 1);
    assert_eq!(snapshot.inline_image_fragments().len(), 4);
    assert_eq!(
        snapshot
            .inline_image_fragments()
            .iter()
            .map(|fragment| {
                (
                    fragment.row,
                    fragment.column,
                    fragment.destination_x,
                    fragment.destination_y,
                    fragment.destination_width,
                    fragment.destination_height,
                    fragment.source_x,
                    fragment.source_y,
                    fragment.source_width,
                    fragment.source_height,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (0, 0, 0, 0, 8, 16, 0, 0, 1, 1),
            (0, 1, 0, 0, 8, 16, 1, 0, 1, 1),
            (1, 0, 0, 0, 8, 16, 0, 1, 1, 1),
            (1, 1, 0, 0, 8, 16, 1, 1, 1, 1),
        ]
    );

    let mut target = vec![0; 2 * 2 * 4];
    PixelRenderer::default().render(&snapshot, &mut target, 2, 2, 1, 1);
    assert_eq!(pixel_at(&target, 2, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 2, 1, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 2, 0, 1), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&target, 2, 1, 1), [255, 255, 255, 255]);
}

#[test]
fn graphics_fragment_renderer_preserves_nonzero_target_offsets() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 3));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=78,f=24,s=2,v=2,c=2,r=2,X=7,Y=15;/wAAAP8AAAD/////\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.inline_image_fragments().len(), 4);
    assert_eq!(
        snapshot.inline_image_fragments()[0].clone(),
        RenderInlineImageFragment {
            parent_image_index: 0,
            cell_attachment: true,
            row: 0,
            column: 0,
            source_row: 0,
            source_column: 0,
            destination_x: 7,
            destination_y: 15,
            destination_width: 8,
            destination_height: 16,
            source_x: 0,
            source_y: 0,
            source_width: 1,
            source_height: 1,
            sampling_source_x: 0,
            sampling_source_y: 0,
            sampling_source_width: 2,
            sampling_source_height: 2,
            source_destination_x: 0,
            source_destination_y: 0,
            source_destination_width: 16,
            source_destination_height: 32,
        }
    );

    let mut target = vec![0; 24 * 48 * 4];
    PixelRenderer::default().render(&snapshot, &mut target, 24, 48, 8, 16);
    assert_eq!(pixel_at(&target, 24, 7, 15), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 24, 14, 15), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 24, 15, 15), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 24, 7, 30), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 24, 7, 31), [0, 0, 255, 255]);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this scenario intentionally covers a complete mutation and deletion lifecycle"
)]
fn target_offset_cell_attachment_mutation_and_deletion_follow_runtime_geometry() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 3));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=179,f=24,s=2,v=2,c=2,r=2,X=7,Y=15;/wAAAP8AAAD/////\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let mut snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.inline_image_fragments.len(), 4);
    assert!(
        snapshot
            .inline_image_fragments
            .iter()
            .all(|attachment| attachment.cell_attachment)
    );
    snapshot
        .inline_image_fragments
        .retain(|attachment| !(attachment.source_row == 1 && attachment.source_column == 0));
    snapshot
        .inline_image_fragments
        .iter_mut()
        .find(|attachment| attachment.source_row == 0 && attachment.source_column == 1)
        .unwrap()
        .column = 2;

    let renderer = PixelRenderer::default();
    for (cell_width, cell_height) in [(8, 16), (10, 20)] {
        let target_width = cell_width * 4;
        let target_height = cell_height * 3;
        let target_width_usize = usize::try_from(target_width).unwrap();
        let cell_width_usize = usize::try_from(cell_width).unwrap();
        let cell_height_usize = usize::try_from(cell_height).unwrap();
        let geometry = RenderGeometry::new(target_width, target_height, cell_width, cell_height);
        let mut full_target = vec![0; usize::try_from(target_width * target_height * 4).unwrap()];
        renderer.render(
            &snapshot,
            &mut full_target,
            target_width,
            target_height,
            cell_width,
            cell_height,
        );
        assert_eq!(
            pixel_at(&full_target, target_width_usize, 7, 15),
            [255, 0, 0, 255]
        );
        assert_eq!(
            pixel_at(
                &full_target,
                target_width_usize,
                cell_width_usize.saturating_add(7),
                15
            ),
            [12, 12, 12, 255]
        );
        assert_eq!(
            pixel_at(
                &full_target,
                target_width_usize,
                cell_width_usize.saturating_mul(2).saturating_add(7),
                15,
            ),
            [0, 255, 0, 255]
        );
        assert_eq!(
            pixel_at(
                &full_target,
                target_width_usize,
                cell_width_usize.saturating_add(7),
                cell_height_usize.saturating_add(15),
            ),
            [255, 255, 255, 255]
        );
        assert_eq!(
            pixel_at(
                &full_target,
                target_width_usize,
                7,
                cell_height_usize.saturating_add(15)
            ),
            [12, 12, 12, 255]
        );

        let mut damage_target = vec![0; usize::try_from(target_width * target_height * 4).unwrap()];
        renderer.render_damage(
            &snapshot,
            &[DamageRegion::new(0, 0, 4, 3)],
            &mut damage_target,
            geometry,
        );
        assert_eq!(
            pixel_at(&damage_target, target_width_usize, 7, 15),
            [255, 0, 0, 255]
        );
        assert_eq!(
            pixel_at(
                &damage_target,
                target_width_usize,
                cell_width_usize.saturating_mul(2).saturating_add(7),
                15,
            ),
            [0, 255, 0, 255]
        );
        assert_eq!(
            pixel_at(
                &damage_target,
                target_width_usize,
                cell_width_usize.saturating_add(7),
                cell_height_usize.saturating_add(15),
            ),
            [255, 255, 255, 255]
        );
    }
}

#[test]
fn graphics_fragment_renderer_derives_target_boundaries_from_runtime_geometry() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 3));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=79,f=24,s=2,v=2,c=2,r=2,X=9,Y=19;/wAAAP8AAAD/////\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut target = vec![0; 30 * 60 * 4];
    PixelRenderer::default().render(&snapshot, &mut target, 30, 60, 10, 20);

    assert_eq!(pixel_at(&target, 30, 9, 19), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 30, 18, 19), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 30, 19, 19), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 30, 9, 38), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 30, 9, 39), [0, 0, 255, 255]);
}

#[test]
fn graphics_fragment_snapshot_keeps_parent_data_when_only_offset_fragment_is_in_viewport() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=80,f=24,s=1,v=1,c=1,r=1,Y=16;/wAA\x1b\\");
    terminal.feed(b"\x1b[?25l");

    assert_eq!(terminal.inline_images().len(), 1);
    assert_eq!(terminal.inline_image_fragments().len(), 1);
    let (
        inline_images,
        inline_image_fragments,
        inline_image_parent_origins,
        empty_inline_image_attachment_parents,
        inline_image_attachment_viewport_offsets,
    ) = render_inline_images_from_terminal(&terminal, 1, 1, 3);
    assert_eq!(inline_images.len(), 1);
    assert_eq!(inline_image_fragments.len(), 1);
    assert_eq!(inline_image_fragments[0].row, 0);
    assert_eq!(inline_image_parent_origins, vec![(0, -1)]);

    let snapshot = TerminalRenderSnapshot::from_inline_image_projection((
        inline_images,
        inline_image_fragments,
        inline_image_parent_origins,
        empty_inline_image_attachment_parents,
        inline_image_attachment_viewport_offsets,
    ));
    let mut target = vec![0; 24 * 16 * 4];
    let renderer = PixelRenderer::default();
    renderer.render(&snapshot, &mut target, 24, 16, 8, 16);
    assert_eq!(pixel_at(&target, 24, 0, 0), [255, 0, 0, 255]);

    let mut damage_target = vec![0; 24 * 16 * 4];
    renderer.render_damage(
        &snapshot,
        &[DamageRegion::new(0, 0, 3, 1)],
        &mut damage_target,
        RenderGeometry::new(24, 16, 8, 16),
    );
    assert_eq!(pixel_at(&damage_target, 24, 0, 0), [255, 0, 0, 255]);
}

#[test]
fn graphics_fragment_destinations_are_authoritative_for_full_and_damage_rendering() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 1));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=81,f=24,s=2,v=1,c=2,r=1;/wAAAP8A\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let mut snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.inline_image_fragments.len(), 2);
    snapshot.inline_image_fragments[1].column = 2;

    let renderer = PixelRenderer::default();
    let mut target = vec![0; 24 * 8 * 4];
    renderer.render(&snapshot, &mut target, 24, 8, 8, 8);
    assert_eq!(pixel_at(&target, 24, 0, 0), [255, 0, 0, 255]);
    assert_ne!(pixel_at(&target, 24, 8, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 24, 16, 0), [0, 255, 0, 255]);

    let mut damage_target = vec![0; 24 * 8 * 4];
    renderer.render_damage(
        &snapshot,
        &[DamageRegion::new(2, 0, 1, 1)],
        &mut damage_target,
        RenderGeometry::new(24, 8, 8, 8),
    );
    assert_eq!(pixel_at(&damage_target, 24, 16, 0), [0, 255, 0, 255]);
    assert_ne!(pixel_at(&damage_target, 24, 8, 0), [0, 255, 0, 255]);
}

#[test]
fn cell_attachment_viewport_clips_rows_for_full_and_damage_rendering() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=182,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal).with_viewport(1, 0, 1, 2);
    assert_eq!(snapshot.inline_images.len(), 1);
    assert_eq!(snapshot.inline_image_fragments.len(), 2);

    let renderer = PixelRenderer::default();
    let mut full_target = vec![0; 16 * 24 * 4];
    renderer.render(&snapshot, &mut full_target, 16, 24, 8, 8);
    assert_eq!(pixel_at(&full_target, 16, 0, 8), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&full_target, 16, 8, 8), [0, 255, 0, 255]);
    assert_ne!(pixel_at(&full_target, 16, 0, 16), [0, 0, 255, 255]);
    assert_ne!(pixel_at(&full_target, 16, 8, 16), [255, 255, 255, 255]);

    let mut damage_target = vec![0; 16 * 24 * 4];
    renderer.render_damage(
        &snapshot,
        &[DamageRegion::new(0, 1, 2, 1)],
        &mut damage_target,
        RenderGeometry::new(16, 24, 8, 8),
    );
    assert_eq!(pixel_at(&damage_target, 16, 0, 8), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&damage_target, 16, 8, 8), [0, 255, 0, 255]);
    assert_ne!(pixel_at(&damage_target, 16, 0, 16), [0, 0, 255, 255]);
    assert_ne!(pixel_at(&damage_target, 16, 8, 16), [255, 255, 255, 255]);

    let column_clipped = TerminalRenderSnapshot::from_terminal(&terminal).with_viewport(1, 1, 1, 1);
    assert_eq!(column_clipped.inline_images.len(), 1);
    assert_eq!(column_clipped.inline_image_fragments.len(), 1);
    let mut column_full_target = vec![0; 24 * 24 * 4];
    renderer.render(&column_clipped, &mut column_full_target, 24, 24, 8, 8);
    assert_eq!(pixel_at(&column_full_target, 24, 8, 8), [255, 0, 0, 255]);
    assert_ne!(pixel_at(&column_full_target, 24, 16, 8), [0, 255, 0, 255]);

    let mut column_damage_target = vec![0; 24 * 24 * 4];
    renderer.render_damage(
        &column_clipped,
        &[DamageRegion::new(1, 1, 1, 1)],
        &mut column_damage_target,
        RenderGeometry::new(24, 24, 8, 8),
    );
    assert_eq!(pixel_at(&column_damage_target, 24, 8, 8), [255, 0, 0, 255]);
    assert_ne!(pixel_at(&column_damage_target, 24, 16, 8), [0, 255, 0, 255]);
}

#[test]
fn cell_attachment_viewport_scissors_target_offset_overflow_for_full_and_damage() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=183,f=24,s=2,v=2,c=2,r=2,X=7,Y=15;/wAAAP8AAAD/////\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal).with_viewport(1, 1, 1, 1);
    assert_eq!(snapshot.inline_images.len(), 1);
    assert_eq!(snapshot.inline_image_fragments.len(), 1);

    let renderer = PixelRenderer::default();
    let geometry = RenderGeometry::new(24, 48, 8, 16);
    let mut full_target = vec![0; 24 * 48 * 4];
    renderer.render(&snapshot, &mut full_target, 24, 48, 8, 16);
    assert_eq!(pixel_at(&full_target, 24, 15, 31), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&full_target, 24, 16, 31), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&full_target, 24, 15, 32), [12, 12, 12, 255]);

    let mut damage_target = vec![0; 24 * 48 * 4];
    renderer.render_damage(
        &snapshot,
        &[DamageRegion::new(1, 1, 2, 2)],
        &mut damage_target,
        geometry,
    );
    assert_eq!(pixel_at(&damage_target, 24, 15, 31), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&damage_target, 24, 16, 31), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&damage_target, 24, 15, 32), [12, 12, 12, 255]);
}

#[test]
fn cell_attachment_snapshot_deletion_is_geometry_independent_for_full_and_damage_rendering() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=181,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let mut snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.inline_image_fragments.len(), 4);
    snapshot
        .inline_image_fragments
        .retain(|attachment| !(attachment.source_row == 0 && attachment.source_column == 1));
    assert_eq!(snapshot.inline_image_fragments.len(), 3);

    let renderer = PixelRenderer::default();
    for (cell_width, cell_height) in [(8, 16), (10, 20)] {
        let target_width = cell_width * 2;
        let target_height = cell_height * 2;
        let target_width_usize = usize::try_from(target_width).unwrap();
        let cell_width_usize = usize::try_from(cell_width).unwrap();
        let cell_height_usize = usize::try_from(cell_height).unwrap();
        let geometry = RenderGeometry::new(target_width, target_height, cell_width, cell_height);

        let mut full_target = vec![0; usize::try_from(target_width * target_height * 4).unwrap()];
        renderer.render(
            &snapshot,
            &mut full_target,
            target_width,
            target_height,
            cell_width,
            cell_height,
        );
        assert_eq!(
            pixel_at(&full_target, target_width_usize, 0, 0),
            [255, 0, 0, 255]
        );
        assert_eq!(
            pixel_at(&full_target, target_width_usize, cell_width_usize, 0),
            [12, 12, 12, 255]
        );
        assert_eq!(
            pixel_at(&full_target, target_width_usize, 0, cell_height_usize),
            [0, 0, 255, 255]
        );
        assert_eq!(
            pixel_at(
                &full_target,
                target_width_usize,
                cell_width_usize,
                cell_height_usize,
            ),
            [255, 255, 255, 255]
        );

        let mut damage_target = vec![0; usize::try_from(target_width * target_height * 4).unwrap()];
        renderer.render_damage(
            &snapshot,
            &[DamageRegion::new(0, 0, 2, 2)],
            &mut damage_target,
            geometry,
        );
        assert_eq!(
            pixel_at(&damage_target, target_width_usize, 0, 0),
            [255, 0, 0, 255]
        );
        assert_eq!(
            pixel_at(&damage_target, target_width_usize, cell_width_usize, 0),
            [12, 12, 12, 255]
        );
        assert_eq!(
            pixel_at(&damage_target, target_width_usize, 0, cell_height_usize),
            [0, 0, 255, 255]
        );
        assert_eq!(
            pixel_at(
                &damage_target,
                target_width_usize,
                cell_width_usize,
                cell_height_usize,
            ),
            [255, 255, 255, 255]
        );
    }
}

#[test]
fn graphics_fragment_viewport_retains_runtime_boundary_candidate_without_default_fragment() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=82,f=24,s=1,v=1,c=1,r=1,Y=10;/wAA\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let (
        inline_images,
        inline_image_fragments,
        inline_image_parent_origins,
        empty_inline_image_attachment_parents,
        inline_image_attachment_viewport_offsets,
    ) = render_inline_images_from_terminal(&terminal, 1, 1, 3);
    assert_eq!(inline_images.len(), 1);
    assert_eq!(inline_image_fragments.len(), 1);
    assert_eq!(inline_image_parent_origins, vec![(0, -1)]);

    let snapshot = TerminalRenderSnapshot::from_inline_image_projection((
        inline_images,
        inline_image_fragments,
        inline_image_parent_origins,
        empty_inline_image_attachment_parents,
        inline_image_attachment_viewport_offsets,
    ));
    let mut target = vec![0; 24 * 20 * 4];
    PixelRenderer::default().render(&snapshot, &mut target, 24, 20, 8, 20);
    assert_eq!(pixel_at(&target, 24, 0, 0), [255, 0, 0, 255]);
}

#[test]
fn graphics_fragment_overlay_sort_keeps_each_parent_origin_paired() {
    fn snapshot_at(column: u16, image_id: u32) -> TerminalRenderSnapshot {
        let mut terminal = Terminal::new(TerminalSize::new(3, 1));
        terminal.feed(format!("\x1b[1;{}H", column + 1).as_bytes());
        terminal.feed(
            format!("\x1b_Ga=T,C=1,q=1,i={image_id},f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\").as_bytes(),
        );
        TerminalRenderSnapshot::from_terminal(&terminal)
    }

    let snapshot = snapshot_at(2, 83)
        .with_overlay_snapshot(snapshot_at(0, 84))
        .with_overlay_snapshot(snapshot_at(1, 85));

    assert_eq!(
        snapshot
            .inline_images
            .iter()
            .map(|image| image.column)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        snapshot.inline_image_parent_origins,
        vec![(0, 0), (1, 0), (2, 0)]
    );
}

#[test]
fn graphics_fragment_viewport_does_not_draw_default_boundary_false_positive() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=86,f=24,s=1,v=1,c=1,r=1,Y=16;/wAA\x1b\\");
    terminal.feed(b"\x1b[?25l");

    let (
        inline_images,
        inline_image_fragments,
        inline_image_parent_origins,
        empty_inline_image_attachment_parents,
        inline_image_attachment_viewport_offsets,
    ) = render_inline_images_from_terminal(&terminal, 1, 1, 3);
    assert_eq!(inline_images.len(), 1);
    assert_eq!(inline_image_fragments.len(), 1);

    let snapshot = TerminalRenderSnapshot::from_inline_image_projection((
        inline_images,
        inline_image_fragments,
        inline_image_parent_origins,
        empty_inline_image_attachment_parents,
        inline_image_attachment_viewport_offsets,
    ));
    let mut target = vec![0; 24 * 8 * 4];
    PixelRenderer::default().render(&snapshot, &mut target, 24, 8, 8, 8);
    assert_ne!(pixel_at(&target, 24, 0, 0), [255, 0, 0, 255]);
}

#[test]
fn render_snapshot_places_inline_images_after_scrollback_exists() {
    let mut terminal = Terminal::new(TerminalSize::new(8, 2));
    terminal.feed(b"one\r\ntwo\r\n");
    terminal.feed(b"\x1b]1337;File=inline=1:QQ==\x07");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert_eq!(snapshot.inline_images().len(), 1);
    assert_eq!(snapshot.inline_images()[0].row, 1);
    assert_eq!(snapshot.inline_images()[0].column, 0);
}

#[test]
fn render_snapshot_can_view_inline_images_in_scrollback() {
    let mut terminal = Terminal::new(TerminalSize::new(8, 2));
    terminal.feed(b"\x1b]1337;File=inline=1:QQ==\x07one\r\ntwo\r\n");

    let live_snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let scrolled_snapshot = TerminalRenderSnapshot::from_terminal_viewport(&terminal, 1);

    assert!(live_snapshot.inline_images().is_empty());
    assert_eq!(scrolled_snapshot.inline_images().len(), 1);
    assert_eq!(scrolled_snapshot.inline_images()[0].row, 0);
}

#[test]
fn render_snapshot_can_apply_inverse_overlay() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 1));
    terminal.feed(b"abc");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal)
        .with_inverse_overlay(|row, column| row == 0 && column == 1);

    assert!(!snapshot.cells()[0].inverse);
    assert!(snapshot.cells()[1].inverse);
    assert!(!snapshot.cells()[2].inverse);
}

#[test]
fn render_snapshot_blends_selection_background_alpha_over_cell_background() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[48;2;10;20;30mA");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal).with_selection_colors_overlay(
        |row, column| row == 0 && column == 0,
        Some(None),
        Some(Color::Rgba(110, 120, 130, 127)),
    );

    assert_eq!(snapshot.cells()[0].background, Color::Rgb(59, 69, 79));
}

#[test]
fn render_snapshot_can_offset_rows_and_overlay_cells() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 1));
    terminal.feed(b"abc");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal)
        .with_row_offset(1)
        .with_overlay_cells([RenderCell::new(0, 0, "T")])
        .with_overlay_cells([RenderCell::new(1, 0, "O")]);

    assert_eq!(snapshot_char(&snapshot, 0, 0), Some('T'));
    assert_eq!(snapshot_char(&snapshot, 1, 0), Some('O'));
    assert_eq!(snapshot_char(&snapshot, 1, 2), Some('c'));
}

#[test]
fn render_snapshot_can_overlay_another_snapshot_with_inline_images() {
    let mut base_terminal = Terminal::new(TerminalSize::new(4, 1));
    base_terminal.feed(b"base");
    let mut overlay_terminal = Terminal::new(TerminalSize::new(4, 1));
    overlay_terminal.feed(b"\x1b]1337;File=inline=1:QQ==\x07");

    let snapshot = TerminalRenderSnapshot::from_terminal(&base_terminal).with_overlay_snapshot(
        TerminalRenderSnapshot::from_terminal(&overlay_terminal).with_viewport(2, 3, 1, 4),
    );

    assert_eq!(snapshot.inline_images().len(), 1);
    assert_eq!(snapshot.inline_images()[0].row, 2);
    assert_eq!(snapshot.inline_images()[0].column, 3);
}

#[test]
fn render_snapshot_can_clip_and_position_viewport() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"abcd\r\nefgh");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal).with_viewport(3, 5, 1, 2);

    assert_eq!(snapshot.cells().len(), 2);
    assert_eq!(snapshot_char(&snapshot, 3, 5), Some('a'));
    assert_eq!(snapshot_char(&snapshot, 3, 6), Some('b'));
    assert_eq!(snapshot_char(&snapshot, 4, 5), None);
    assert_eq!(snapshot_char(&snapshot, 3, 7), None);
}

#[test]
fn bounded_scroll_blanks_final_cell_attachment_for_full_and_damage_rendering() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"\x1b[1;2H");
    feed_red_inline_png(&mut terminal, "width=1;height=1");
    terminal.feed(b"\x1b[?25l\x1b[?69h\x1b[2;3s\x1b[1;2r\x1b[2;2H");
    terminal.take_damage();

    terminal.feed(b"\n");
    let damage = terminal.take_damage();
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.inline_images().len(), 1);
    assert!(snapshot.inline_image_fragments().is_empty());

    let renderer = PixelRenderer::default();
    let geometry = RenderGeometry::new(32, 16, 8, 8);
    let mut full_target = vec![0; 32 * 16 * 4];
    renderer.render(&snapshot, &mut full_target, 32, 16, 8, 8);
    assert_eq!(pixel_at(&full_target, 32, 8, 0), [12, 12, 12, 255]);

    let mut damage_target = vec![0; 32 * 16 * 4];
    renderer.render_damage(&snapshot, &damage, &mut damage_target, geometry);
    assert_eq!(pixel_at(&damage_target, 32, 8, 0), [12, 12, 12, 255]);
}

#[test]
fn bounded_scroll_moves_iterm_attachment_cells_using_decoded_payload_dimensions() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 3));
    terminal.feed(b"\x1b[2;2H");
    feed_red_inline_png(&mut terminal, "width=2;height=2");
    terminal.feed(b"\x1b[?25l\x1b[?69h\x1b[2;3s\x1b[1;3r\x1b[3;2H");
    terminal.take_damage();

    terminal.feed(b"\n");
    let damage = terminal.take_damage();
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.inline_image_fragments().len(), 4);
    assert_eq!(
        snapshot
            .inline_image_fragments()
            .iter()
            .map(|fragment| (fragment.row, fragment.column))
            .collect::<Vec<_>>(),
        vec![(0, 1), (0, 2), (1, 1), (1, 2)]
    );

    let renderer = PixelRenderer::default();
    let geometry = RenderGeometry::new(32, 24, 8, 8);
    let mut full_target = vec![0; 32 * 24 * 4];
    renderer.render(&snapshot, &mut full_target, 32, 24, 8, 8);
    assert_eq!(pixel_at(&full_target, 32, 8, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&full_target, 32, 16, 8), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&full_target, 32, 8, 16), [12, 12, 12, 255]);

    let mut damage_target = vec![0; 32 * 24 * 4];
    renderer.render_damage(&snapshot, &damage, &mut damage_target, geometry);
    assert_eq!(pixel_at(&damage_target, 32, 8, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&damage_target, 32, 16, 8), [255, 0, 0, 255]);
}

#[test]
fn bounded_scroll_damage_clears_offset_attachment_pixels_outside_lr() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 3));
    terminal.feed(b"\x1b[2;3H");
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=207,f=24,s=1,v=1,c=1,r=1,X=7,Y=7;/wAA\x1b\\");
    terminal.feed(b"\x1b[?25l\x1b[?69h\x1b[2;3s\x1b[1;3r\x1b[3;3H");
    terminal.take_damage();

    let renderer = PixelRenderer::default();
    let geometry = RenderGeometry::new(32, 24, 8, 8);
    let mut target = vec![0; 32 * 24 * 4];
    renderer.render(
        &TerminalRenderSnapshot::from_terminal(&terminal),
        &mut target,
        32,
        24,
        8,
        8,
    );
    assert_eq!(pixel_at(&target, 32, 24, 15), [255, 0, 0, 255]);

    terminal.feed(b"\n");
    let damage = terminal.take_damage();
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut full_target = vec![0; 32 * 24 * 4];
    renderer.render(&snapshot, &mut full_target, 32, 24, 8, 8);
    renderer.render_damage(&snapshot, &damage, &mut target, geometry);

    assert_eq!(target, full_target);
    assert_eq!(pixel_at(&target, 32, 24, 7), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 32, 24, 15), [12, 12, 12, 255]);
}

#[test]
fn bounded_scroll_damage_clears_offset_attachment_pixels_after_blank() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"\x1b[1;2H");
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=208,f=24,s=1,v=1,c=1,r=1,X=7,Y=7;/wAA\x1b\\");
    terminal.feed(b"\x1b[?25l\x1b[?69h\x1b[2;3s\x1b[1;2r\x1b[2;2H");
    terminal.take_damage();

    let renderer = PixelRenderer::default();
    let geometry = RenderGeometry::new(32, 16, 8, 8);
    let mut target = vec![0; 32 * 16 * 4];
    renderer.render(
        &TerminalRenderSnapshot::from_terminal(&terminal),
        &mut target,
        32,
        16,
        8,
        8,
    );
    assert_eq!(pixel_at(&target, 32, 15, 7), [255, 0, 0, 255]);

    terminal.feed(b"\n");
    let damage = terminal.take_damage();
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.inline_image_fragments().is_empty());
    assert!(snapshot.empty_inline_image_attachment_parents.contains(&0));
    let mut full_target = vec![0; 32 * 16 * 4];
    renderer.render(&snapshot, &mut full_target, 32, 16, 8, 8);
    renderer.render_damage(&snapshot, &damage, &mut target, geometry);

    assert_eq!(target, full_target);
    assert_eq!(pixel_at(&target, 32, 15, 7), [12, 12, 12, 255]);
}

#[test]
fn bounded_ich_damage_moves_offset_attachment_pixels_for_full_and_damage_rendering() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"\x1b[1;2H");
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=209,f=24,s=1,v=1,c=1,r=1,X=7,Y=7;/wAA\x1b\\");
    terminal.feed(b"\x1b[?25l\x1b[?69h\x1b[2;3s\x1b[1;2r\x1b[1;2H");
    terminal.take_damage();

    let renderer = PixelRenderer::default();
    let geometry = RenderGeometry::new(32, 16, 8, 8);
    let mut target = vec![0; 32 * 16 * 4];
    renderer.render(
        &TerminalRenderSnapshot::from_terminal(&terminal),
        &mut target,
        32,
        16,
        8,
        8,
    );
    assert_eq!(pixel_at(&target, 32, 15, 7), [255, 0, 0, 255]);

    terminal.feed(b"\x1b[@");
    let damage = terminal.take_damage();
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut full_target = vec![0; 32 * 16 * 4];
    renderer.render(&snapshot, &mut full_target, 32, 16, 8, 8);
    renderer.render_damage(&snapshot, &damage, &mut target, geometry);

    assert_eq!(target, full_target);
    assert_eq!(pixel_at(&target, 32, 15, 7), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&target, 32, 23, 7), [255, 0, 0, 255]);
}

#[test]
fn bounded_dch_damage_moves_offset_attachment_pixels_for_full_and_damage_rendering() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"\x1b[1;3H");
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=210,f=24,s=1,v=1,c=1,r=1,X=7,Y=7;/wAA\x1b\\");
    terminal.feed(b"\x1b[?25l\x1b[?69h\x1b[2;3s\x1b[1;2r\x1b[1;2H");
    terminal.take_damage();

    let renderer = PixelRenderer::default();
    let geometry = RenderGeometry::new(32, 16, 8, 8);
    let mut target = vec![0; 32 * 16 * 4];
    renderer.render(
        &TerminalRenderSnapshot::from_terminal(&terminal),
        &mut target,
        32,
        16,
        8,
        8,
    );
    assert_eq!(pixel_at(&target, 32, 23, 7), [255, 0, 0, 255]);

    terminal.feed(b"\x1b[P");
    let damage = terminal.take_damage();
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut full_target = vec![0; 32 * 16 * 4];
    renderer.render(&snapshot, &mut full_target, 32, 16, 8, 8);
    renderer.render_damage(&snapshot, &damage, &mut target, geometry);

    assert_eq!(target, full_target);
    assert_eq!(pixel_at(&target, 32, 23, 7), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&target, 32, 15, 7), [255, 0, 0, 255]);
}

#[test]
fn bounded_ich_moves_decoded_iterm_attachment_cells_for_png_jpeg_and_gif() {
    for feed_image in [
        feed_red_inline_png as fn(&mut Terminal, &str),
        feed_red_inline_jpeg,
        feed_red_inline_gif,
    ] {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"\x1b[1;2H");
        feed_image(&mut terminal, "width=1;height=1");
        terminal.feed(b"\x1b[?25l\x1b[?69h\x1b[2;3s\x1b[1;2r\x1b[1;2H\x1b[@");

        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        assert_eq!(
            snapshot
                .inline_image_fragments()
                .iter()
                .map(|fragment| (fragment.row, fragment.column))
                .collect::<Vec<_>>(),
            vec![(0, 2)]
        );

        let mut target = vec![0; 32 * 16 * 4];
        PixelRenderer::default().render(&snapshot, &mut target, 32, 16, 8, 8);
        assert_eq!(pixel_at(&target, 32, 8, 0), [12, 12, 12, 255]);
        assert_ne!(pixel_at(&target, 32, 16, 0), [12, 12, 12, 255]);
    }
}

#[test]
fn bounded_scroll_moves_jpeg_attachment_cells_using_decoded_payload_dimensions() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 3));
    terminal.feed(b"\x1b[2;2H");
    feed_red_inline_jpeg(&mut terminal, "width=2;height=2");
    terminal.feed(b"\x1b[?25l\x1b[?69h\x1b[2;3s\x1b[1;3r\x1b[3;2H");
    terminal.take_damage();
    let initial_snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    terminal.feed(b"\n");
    let damage = terminal.take_damage();
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.inline_image_fragments().len(), 4);

    let renderer = PixelRenderer::default();
    let geometry = RenderGeometry::new(32, 24, 8, 8);
    let mut full_target = vec![0; 32 * 24 * 4];
    renderer.render(&snapshot, &mut full_target, 32, 24, 8, 8);
    assert_eq!(pixel_at(&full_target, 32, 8, 0), [254, 0, 0, 255]);
    assert_eq!(pixel_at(&full_target, 32, 16, 8), [254, 0, 0, 255]);
    assert_eq!(pixel_at(&full_target, 32, 8, 16), [12, 12, 12, 255]);

    let mut damage_target = vec![0; 32 * 24 * 4];
    renderer.render(&initial_snapshot, &mut damage_target, 32, 24, 8, 8);
    renderer.render_damage(&snapshot, &damage, &mut damage_target, geometry);
    assert_eq!(damage_target, full_target);
}

#[test]
fn bounded_scroll_blanks_gif_attachment_cells_using_decoded_payload_dimensions() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"\x1b[1;2H");
    feed_red_inline_gif(&mut terminal, "width=1;height=1");
    terminal.feed(b"\x1b[?25l\x1b[?69h\x1b[2;3s\x1b[1;2r\x1b[2;2H");
    terminal.take_damage();
    let initial_snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    terminal.feed(b"\n");
    let damage = terminal.take_damage();
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.inline_image_fragments().is_empty());

    let renderer = PixelRenderer::default();
    let geometry = RenderGeometry::new(32, 16, 8, 8);
    let mut full_target = vec![0; 32 * 16 * 4];
    renderer.render(&snapshot, &mut full_target, 32, 16, 8, 8);
    assert_eq!(pixel_at(&full_target, 32, 8, 0), [12, 12, 12, 255]);

    let mut damage_target = vec![0; 32 * 16 * 4];
    renderer.render(&initial_snapshot, &mut damage_target, 32, 16, 8, 8);
    renderer.render_damage(&snapshot, &damage, &mut damage_target, geometry);
    assert_eq!(damage_target, full_target);
}

#[test]
fn render_snapshot_updates_cells_from_damage_regions() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 1));
    terminal.feed(b"abc");
    terminal.take_damage();
    let mut snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    terminal.feed(b"\rZ");
    let damage = terminal.take_damage();

    snapshot.update_from_terminal_damage(&terminal, &damage);

    assert_eq!(snapshot_char(&snapshot, 0, 0), Some('Z'));
    assert_eq!(snapshot_char(&snapshot, 0, 1), Some('b'));
    assert_eq!(snapshot_char(&snapshot, 0, 2), Some('c'));
}

#[test]
fn render_snapshot_removes_cells_cleared_by_damage_regions() {
    let mut terminal = Terminal::new(TerminalSize::new(3, 1));
    terminal.feed(b"abc");
    terminal.take_damage();
    let mut snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    terminal.feed(b"\r ");
    let damage = terminal.take_damage();

    snapshot.update_from_terminal_damage(&terminal, &damage);

    assert_eq!(snapshot_char(&snapshot, 0, 0), None);
    assert_eq!(snapshot_char(&snapshot, 0, 1), Some('b'));
    assert_eq!(snapshot_char(&snapshot, 0, 2), Some('c'));
}

#[test]
fn pixel_renderer_draws_glyph_foreground_pixels() {
    let mut grid = TerminalGrid::new(TerminalSize::new(1, 1));
    let mut cell = Cell::with_char('A');
    cell.foreground = Color::Rgb(255, 0, 0);
    grid.set(0, 0, cell);
    let snapshot = TerminalRenderSnapshot::from_grid(&grid);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert!(
        target
            .chunks_exact(4)
            .any(|pixel| pixel == [255, 0, 0, 255]),
        "renderer did not draw a red glyph pixel"
    );
}

#[test]
fn pixel_renderer_draws_iterm_inline_png_image_payload() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_red_inline_png(&mut terminal, "width=1;height=1");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_draws_iterm_inline_jpeg_image_payload() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_red_inline_jpeg(&mut terminal, "width=1;height=1");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [254, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [254, 0, 0, 255]);
}

#[test]
fn pixel_renderer_draws_iterm_inline_gif_first_frame() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_red_inline_gif(&mut terminal, "width=1;height=1");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_draws_iterm_inline_gif_animation_frame() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_red_green_inline_gif(&mut terminal, "width=1;height=1");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::with_animation_frame(1);
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [0, 255, 0, 255]);

    let renderer = PixelRenderer::with_animation_elapsed_ms(250);
    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_selects_iterm_inline_gif_frame_by_elapsed_time() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_red_green_inline_gif(&mut terminal, "width=1;height=1");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::with_animation_elapsed_ms(150);
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [0, 255, 0, 255]);
}

#[test]
fn pixel_renderer_draws_kitty_rgb_direct_inline_image() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_red_kitty_rgb_image(&mut terminal);
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_draws_compressed_kitty_rgb_direct_inline_image() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_compressed_red_kitty_rgb_image(&mut terminal);
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_draws_kitty_rgb_simple_file_transfer() {
    let file = KittyTestFile::new(&[255, 0, 0]);
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_kitty_rgb_file_image(&mut terminal, &file.path, "");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_draws_kitty_rgb_simple_file_transfer_slice() {
    let file = KittyTestFile::new(&[0, 0, 255, 255, 0, 0]);
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_kitty_rgb_file_image(&mut terminal, &file.path, ",O=3,S=3");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_draws_kitty_rgb_temporary_file_transfer_and_deletes_safe_temp_file() {
    let file = KittyTestFile::new_with_prefix("tty-graphics-protocol-rssh", &[255, 0, 0]);
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_kitty_rgb_temporary_file_image(&mut terminal, &file.path, "");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
    assert!(
        !file.path.exists(),
        "safe kitty temporary file should be deleted after reading"
    );
}

#[test]
fn pixel_renderer_preserves_kitty_temporary_file_without_safe_name() {
    let file = KittyTestFile::new(&[255, 0, 0]);
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_kitty_rgb_temporary_file_image(&mut terminal, &file.path, "");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
    assert!(
        file.path.exists(),
        "unsafe kitty temporary file name should not be deleted"
    );
}

#[test]
fn pixel_renderer_draws_chunked_kitty_rgb_direct_inline_image() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_chunked_red_green_kitty_rgb_image(&mut terminal);
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 0), [0, 255, 0, 255]);
}

#[test]
fn pixel_renderer_draws_basic_sixel_image() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 2));
    terminal.feed(b"\x1b[?25l");
    terminal.feed(b"\x1bPq\"1;1;1;6#1;2;100;0;0#1~\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 0, 5), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 1, 0), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&target, 8, 0, 6), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_draws_sixel_repeat_and_newline() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 2));
    terminal.feed(b"\x1b[?25l");
    terminal.feed(b"\x1bPq\"1;1;2;12#1;2;100;0;0#1!2@-!2@\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 16 * 4];

    renderer.render(&snapshot, &mut target, 8, 16, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 1, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 0, 6), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 1, 6), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 2, 0), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_draws_sixel_hls_color_definition() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 2));
    terminal.feed(b"\x1b[?25l");
    terminal.feed(b"\x1bPq#1;1;240;50;100#1~\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 0, 5), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 1, 0), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_draws_kitty_horizontal_source_rectangle() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=T,f=24,s=2,v=1,c=1,r=1,x=1,w=1;/wAAAP8A\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [0, 255, 0, 255]);
}

#[test]
fn pixel_renderer_draws_kitty_vertical_source_rectangle() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=T,f=24,s=1,v=2,c=1,r=1,y=1,h=1;/wAAAP8A\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [0, 255, 0, 255]);
}

#[test]
fn pixel_renderer_draws_kitty_target_pixel_offset() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=T,f=24,s=1,v=1,c=1,r=1,X=2,Y=3;/wAA\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 16 * 4];

    renderer.render(&snapshot, &mut target, 16, 16, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 0), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&target, 16, 8, 3), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 2, 10), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 1, 3), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&target, 16, 2, 2), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_draws_stored_kitty_source_rectangle() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=2,v=1,c=1,r=1;/wAAAP8A\x1b\\");
    terminal.feed(b"\x1b_Ga=p,i=7,x=1,w=1\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [0, 255, 0, 255]);
}

#[test]
fn pixel_renderer_draws_stored_kitty_target_pixel_offset() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
    terminal.feed(b"\x1b_Ga=p,i=7,X=2,Y=3\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 16 * 4];

    renderer.render(&snapshot, &mut target, 16, 16, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 0), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&target, 16, 8, 3), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 2, 10), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 1, 3), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&target, 16, 2, 2), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_draws_stored_kitty_rgb_direct_inline_image() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_stored_red_kitty_rgb_image(&mut terminal);
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_stacks_kitty_images_by_z_index() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_overlapping_kitty_rgb_images(&mut terminal, 5, 1);
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_uses_kitty_image_id_as_same_z_index_tiebreaker() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_overlapping_kitty_rgb_images_high_id_first(&mut terminal);
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [0, 255, 0, 255]);
}

#[test]
fn pixel_renderer_places_negative_z_kitty_images_below_text() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25lA\x1b[1;1H");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
    terminal.feed(b"\x1b_Ga=p,i=7,z=-1\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 3, 0), [229, 229, 229, 255]);
}

#[test]
fn pixel_renderer_places_extreme_negative_z_kitty_images_below_non_default_backgrounds() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l\x1b[48;2;0;0;255mA\x1b[0m\x1b[1;1H");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
    terminal.feed(b"\x1b_Ga=p,i=7,z=-1073741825\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&target, 8, 3, 0), [229, 229, 229, 255]);
}

#[test]
fn pixel_renderer_places_extreme_negative_z_kitty_images_below_non_default_space_backgrounds() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l\x1b[48;2;0;0;255m \x1b[0m\x1b[1;1H");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
    terminal.feed(b"\x1b_Ga=p,i=7,z=-1073741825\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [0, 0, 255, 255]);
}

#[test]
fn pixel_renderer_omits_deleted_kitty_placements() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_stored_red_kitty_rgb_image(&mut terminal);
    terminal.feed(b"\x1b_Ga=d\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_draws_inline_image_from_damage_region() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_red_inline_png(&mut terminal, "width=1;height=1");
    let damage = terminal.take_damage();
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render_damage(
        &snapshot,
        &damage,
        &mut target,
        RenderGeometry::new(8, 8, 8, 8),
    );

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_redraws_inline_image_when_damage_hits_covered_cell() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    feed_red_inline_png(&mut terminal, "width=2;height=1");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render_damage(
        &snapshot,
        &[DamageRegion::new(1, 0, 1, 1)],
        &mut target,
        RenderGeometry::new(16, 8, 8, 8),
    );

    assert_eq!(pixel_at(&target, 16, 8, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 15, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_respects_inline_image_pixel_dimensions() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_red_inline_png(&mut terminal, "width=4px;height=2px");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 3, 1), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 4, 1), [12, 12, 12, 255]);
    assert_eq!(pixel_at(&target, 8, 3, 2), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_updates_only_damage_regions() {
    let mut grid = TerminalGrid::new(TerminalSize::new(2, 1));
    let mut first = Cell::with_char('A');
    first.background = Color::Rgb(20, 0, 0);
    grid.set(0, 0, first);
    let mut second = Cell::with_char('B');
    second.background = Color::Rgb(0, 20, 0);
    grid.set(0, 1, second);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(
        &TerminalRenderSnapshot::from_grid(&grid),
        &mut target,
        16,
        8,
        8,
        8,
    );
    let untouched_second_cell = pixel_at(&target, 16, 8, 0);

    let mut cell = Cell::with_char('Z');
    cell.foreground = Color::Rgb(0, 0, 20);
    cell.background = Color::Rgb(0, 0, 20);
    grid.set(0, 0, cell);

    renderer.render_damage(
        &TerminalRenderSnapshot::from_grid(&grid),
        &[DamageRegion::new(0, 0, 1, 1)],
        &mut target,
        RenderGeometry::new(16, 8, 8, 8),
    );

    assert_eq!(pixel_at(&target, 16, 0, 0), [0, 0, 20, 255]);
    assert_eq!(pixel_at(&target, 16, 8, 0), untouched_second_cell);
}

#[test]
fn pixel_renderer_draws_scrollback_scrollbar_at_bottom_for_live_viewport() {
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 32 * 4];

    renderer.render_scrollbar(
        ScrollbackScrollbar::new(3, 1, 0).unwrap(),
        &mut target,
        RenderGeometry::new(16, 32, 8, 8),
    );

    assert_eq!(pixel_at(&target, 16, 15, 0), SCROLLBAR_TRACK_COLOR);
    assert_eq!(pixel_at(&target, 16, 15, 31), SCROLLBAR_THUMB_COLOR);
}

#[test]
fn modern_default_scrollback_scrollbar_uses_deep_blue_surface_colors() {
    assert_eq!(SCROLLBAR_TRACK_COLOR, [0x10, 0x18, 0x27, 0xff]);
    assert_eq!(SCROLLBAR_THUMB_COLOR, [0x47, 0x55, 0x69, 0xff]);
}

#[test]
fn pixel_renderer_moves_scrollback_scrollbar_thumb_up_for_history_viewport() {
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 32 * 4];

    renderer.render_scrollbar(
        ScrollbackScrollbar::new(3, 1, 3).unwrap(),
        &mut target,
        RenderGeometry::new(16, 32, 8, 8),
    );

    assert_eq!(pixel_at(&target, 16, 15, 0), SCROLLBAR_THUMB_COLOR);
    assert_eq!(pixel_at(&target, 16, 15, 31), SCROLLBAR_TRACK_COLOR);
}

#[test]
fn pixel_renderer_uses_half_cell_default_minimum_scrollbar_thumb_height() {
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 32 * 4];

    renderer.render_scrollbar(
        ScrollbackScrollbar::new(1_000, 1, 0).unwrap(),
        &mut target,
        RenderGeometry::new(16, 32, 8, 8),
    );

    assert_eq!(pixel_at(&target, 16, 15, 27), SCROLLBAR_TRACK_COLOR);
    assert_eq!(pixel_at(&target, 16, 15, 28), SCROLLBAR_THUMB_COLOR);
    assert_eq!(pixel_at(&target, 16, 15, 31), SCROLLBAR_THUMB_COLOR);
}

#[test]
fn pixel_renderer_applies_percent_minimum_scrollbar_thumb_height() {
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 32 * 4];

    renderer.render_scrollbar(
        ScrollbackScrollbar::new(1_000, 1, 0)
            .unwrap()
            .with_min_thumb_height_percent(50),
        &mut target,
        RenderGeometry::new(16, 32, 8, 8),
    );

    assert_eq!(pixel_at(&target, 16, 15, 15), SCROLLBAR_TRACK_COLOR);
    assert_eq!(pixel_at(&target, 16, 15, 16), SCROLLBAR_THUMB_COLOR);
    assert_eq!(pixel_at(&target, 16, 15, 31), SCROLLBAR_THUMB_COLOR);
}

#[test]
fn pixel_renderer_scales_point_minimum_scrollbar_thumb_height_by_window_dpi() {
    let mut renderer = PixelRenderer::new();
    renderer.set_window_dpi(144);
    let mut target = vec![0; 16 * 32 * 4];

    renderer.render_scrollbar(
        ScrollbackScrollbar::new(1_000, 1, 0)
            .unwrap()
            .with_min_thumb_height_points(3),
        &mut target,
        RenderGeometry::new(16, 32, 8, 8),
    );

    assert_eq!(pixel_at(&target, 16, 15, 25), SCROLLBAR_TRACK_COLOR);
    assert_eq!(pixel_at(&target, 16, 15, 26), SCROLLBAR_THUMB_COLOR);
    assert_eq!(pixel_at(&target, 16, 15, 31), SCROLLBAR_THUMB_COLOR);
}

#[test]
fn scrollback_scrollbar_maps_pixel_y_to_viewport_offset() {
    let geometry = RenderGeometry::new(8, 100, 1, 1);
    let scrollbar = ScrollbackScrollbar::new(10, 10, 0).unwrap();

    assert_eq!(scrollbar.offset_from_pixel_y(0, geometry), 10);
    assert_eq!(scrollbar.offset_from_pixel_y(99, geometry), 0);
}

#[test]
fn indexed_color_maps_xterm_256_color_palette() {
    assert_eq!(
        super::color_to_rgba(Color::Indexed(16), [1, 2, 3, 255]),
        [0, 0, 0, 255]
    );
    assert_eq!(
        super::color_to_rgba(Color::Indexed(196), [1, 2, 3, 255]),
        [255, 0, 0, 255]
    );
    assert_eq!(
        super::color_to_rgba(Color::Indexed(232), [1, 2, 3, 255]),
        [8, 8, 8, 255]
    );
    assert_eq!(
        super::color_to_rgba(Color::Indexed(255), [1, 2, 3, 255]),
        [238, 238, 238, 255]
    );
}

#[test]
fn color_to_rgba_preserves_terminal_rgba_alpha() {
    assert_eq!(
        super::color_to_rgba(Color::Rgba(1, 2, 3, 4), [9, 9, 9, 255]),
        [1, 2, 3, 4]
    );
}

#[test]
fn pixel_renderer_draws_xterm_256_color_from_terminal_output() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[38;5;196mR");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.cells()[0].foreground, Color::Indexed(196));
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert!(
        target
            .chunks_exact(4)
            .any(|pixel| pixel == [255, 0, 0, 255]),
        "renderer did not draw xterm indexed red"
    );
}

#[test]
fn pixel_renderer_draws_underlined_text() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[4;38;2;255;0;0mA");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].underline);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 7), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_draws_underlines_with_underline_color() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[4;38;2;255;0;0;58;2;0;255;0mA");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.cells()[0].underline_color, Color::Rgb(0, 255, 0));
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 7), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 7), [0, 255, 0, 255]);
    assert!(
        target
            .chunks_exact(4)
            .any(|pixel| pixel == [255, 0, 0, 255]),
        "glyph foreground should still use the foreground color"
    );
}

#[test]
fn pixel_renderer_applies_underline_thickness_override() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[4;38;2;255;0;0m ");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].underline);
    let renderer = PixelRenderer::with_underline_thickness_px(3);
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 5), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 5), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 0, 4), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_applies_underline_position_override() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[4;38;2;255;0;0m ");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].underline);
    let renderer = PixelRenderer::with_underline_position_px(-3);
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 4), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 4), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 0, 7), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_applies_strikethrough_position_override() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[9;38;2;255;0;0m ");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].strikethrough);
    let renderer = PixelRenderer::with_strikethrough_position_cell_fraction_per_mille(250);
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 2), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 2), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 0, 4), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_draws_dotted_underlines_with_gaps() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[4:4;58;2;0;255;0mA");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(
        snapshot.cells()[0].underline_style,
        rssh_terminal::UnderlineStyle::Dotted
    );
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 7), [0, 255, 0, 255]);
    assert_ne!(pixel_at(&target, 16, 1, 7), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 2, 7), [0, 255, 0, 255]);
}

#[test]
fn pixel_renderer_draws_double_underlined_text() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[21;38;2;255;0;0mA");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].double_underline);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 5), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 5), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 0, 7), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_draws_strikethrough_text() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[9;38;2;255;0;0m.");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].strikethrough);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 4), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 4), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_dims_faint_foreground_text() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[2;4;38;2;200;100;50m.");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].faint);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 7), [100, 50, 25, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 7), [100, 50, 25, 255]);
}

#[test]
fn pixel_renderer_hides_concealed_foreground_text() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[8;4;38;2;255;0;0;48;2;3;4;5m.");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].conceal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 7), [3, 4, 5, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 7), [3, 4, 5, 255]);
    assert_eq!(count_pixels(&target, [255, 0, 0, 255]), 0);
}

#[test]
fn pixel_renderer_draws_overlined_text() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[53;38;2;255;0;0m.");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].overline);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 0), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_hides_blinking_foreground_when_phase_is_hidden() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[5;4;38;2;255;0;0;48;2;3;4;5m.");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].blink);
    let renderer = PixelRenderer::with_blink_visible(false);
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(pixel_at(&target, 16, 0, 7), [3, 4, 5, 255]);
    assert_eq!(pixel_at(&target, 16, 7, 7), [3, 4, 5, 255]);
    assert_eq!(count_pixels(&target, [255, 0, 0, 255]), 0);
}

#[test]
fn pixel_renderer_fades_blinking_foreground_toward_background() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[5;38;2;255;0;0;48;2;3;4;5m.");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].blink);
    let renderer = PixelRenderer::with_text_blink_opacity(0.5);
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert!(count_pixels(&target, [128, 2, 2, 255]) > 0);
    assert_eq!(count_pixels(&target, [255, 0, 0, 255]), 0);
}

#[test]
fn pixel_renderer_uses_rapid_text_blink_opacity_for_sgr6_cells() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[6;38;2;255;0;0;48;2;3;4;5m.");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].blink);
    assert!(snapshot.cells()[0].rapid_blink);
    let renderer = PixelRenderer::with_rapid_text_blink_opacity(0.0);
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert_eq!(count_pixels(&target, [255, 0, 0, 255]), 0);
    assert_eq!(count_pixels(&target, [128, 2, 2, 255]), 0);
}

#[test]
fn pixel_renderer_draws_bold_text_with_more_foreground_pixels() {
    let renderer = PixelRenderer::new();
    let mut normal = Terminal::new(TerminalSize::new(2, 1));
    normal.feed(b"\x1b[38;2;255;0;0mA");
    let normal_snapshot = TerminalRenderSnapshot::from_terminal(&normal);
    let mut normal_target = vec![0; 16 * 8 * 4];

    renderer.render(&normal_snapshot, &mut normal_target, 16, 8, 8, 8);

    let mut bold = Terminal::new(TerminalSize::new(2, 1));
    bold.feed(b"\x1b[1;38;2;255;0;0mA");
    let bold_snapshot = TerminalRenderSnapshot::from_terminal(&bold);
    assert!(bold_snapshot.cells()[0].bold);
    let mut bold_target = vec![0; 16 * 8 * 4];

    renderer.render(&bold_snapshot, &mut bold_target, 16, 8, 8, 8);

    assert!(
        count_pixels(&bold_target, [255, 0, 0, 255])
            > count_pixels(&normal_target, [255, 0, 0, 255])
    );
}

#[test]
fn pixel_renderer_brightens_bold_ansi_foreground_by_default() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[1;31mA");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cells()[0].bold);
    assert_eq!(snapshot.cells()[0].foreground, Color::Indexed(1));
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert!(count_pixels(&target, [241, 76, 76, 255]) > 0);
    assert_eq!(count_pixels(&target, [205, 49, 49, 255]), 0);
}

#[test]
fn pixel_renderer_can_disable_bold_ansi_brightening() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[1;31mA");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer =
        PixelRenderer::with_bold_brightens_ansi_colors(RenderBoldBrightensAnsiColors::No);
    let mut target = vec![0; 16 * 8 * 4];

    renderer.render(&snapshot, &mut target, 16, 8, 8, 8);

    assert!(count_pixels(&target, [205, 49, 49, 255]) > 0);
    assert_eq!(count_pixels(&target, [241, 76, 76, 255]), 0);
}

#[test]
fn pixel_renderer_bright_only_ansi_bold_uses_bright_color_without_bold_weight() {
    let renderer =
        PixelRenderer::with_bold_brightens_ansi_colors(RenderBoldBrightensAnsiColors::BrightOnly);
    let mut normal = Terminal::new(TerminalSize::new(2, 1));
    normal.feed(b"\x1b[91mA");
    let normal_snapshot = TerminalRenderSnapshot::from_terminal(&normal);
    let mut normal_target = vec![0; 16 * 8 * 4];

    renderer.render(&normal_snapshot, &mut normal_target, 16, 8, 8, 8);

    let mut bold = Terminal::new(TerminalSize::new(2, 1));
    bold.feed(b"\x1b[1;31mA");
    let bold_snapshot = TerminalRenderSnapshot::from_terminal(&bold);
    assert!(bold_snapshot.cells()[0].bold);
    let mut bold_target = vec![0; 16 * 8 * 4];

    renderer.render(&bold_snapshot, &mut bold_target, 16, 8, 8, 8);

    let normal_bright_pixels = count_pixels(&normal_target, [241, 76, 76, 255]);
    assert!(normal_bright_pixels > 0);
    assert_eq!(
        count_pixels(&bold_target, [241, 76, 76, 255]),
        normal_bright_pixels
    );
    assert_eq!(count_pixels(&bold_target, [205, 49, 49, 255]), 0);
}

#[test]
fn pixel_renderer_slants_italic_text() {
    let renderer = PixelRenderer::new();
    let mut normal = Terminal::new(TerminalSize::new(2, 1));
    normal.feed(b"\x1b[38;2;255;0;0mI");
    let normal_snapshot = TerminalRenderSnapshot::from_terminal(&normal);
    let mut normal_target = vec![0; 16 * 8 * 4];

    renderer.render(&normal_snapshot, &mut normal_target, 16, 8, 8, 8);

    let mut italic = Terminal::new(TerminalSize::new(2, 1));
    italic.feed(b"\x1b[3;38;2;255;0;0mI");
    let italic_snapshot = TerminalRenderSnapshot::from_terminal(&italic);
    assert!(italic_snapshot.cells()[0].italic);
    let mut italic_target = vec![0; 16 * 8 * 4];

    renderer.render(&italic_snapshot, &mut italic_target, 16, 8, 8, 8);

    assert_ne!(italic_target, normal_target);
    assert_eq!(
        count_pixels(&italic_target, [255, 0, 0, 255]),
        count_pixels(&normal_target, [255, 0, 0, 255])
    );
}

#[test]
fn pixel_renderer_offsets_subscript_text_baseline() {
    let renderer = PixelRenderer::new();
    let mut baseline = Terminal::new(TerminalSize::new(2, 1));
    baseline.feed(b"\x1b[38;2;255;0;0mA");
    let baseline_snapshot = TerminalRenderSnapshot::from_terminal(&baseline);
    assert_eq!(
        baseline_snapshot.cells()[0].vertical_align,
        rssh_terminal::VerticalAlign::Baseline
    );
    let mut baseline_target = vec![0; 16 * 16 * 4];

    renderer.render(&baseline_snapshot, &mut baseline_target, 16, 16, 8, 16);

    let mut subscript = Terminal::new(TerminalSize::new(2, 1));
    subscript.feed(b"\x1b[74;38;2;255;0;0mA");
    let subscript_snapshot = TerminalRenderSnapshot::from_terminal(&subscript);
    assert_eq!(
        subscript_snapshot.cells()[0].vertical_align,
        rssh_terminal::VerticalAlign::Subscript
    );
    let mut subscript_target = vec![0; 16 * 16 * 4];

    renderer.render(&subscript_snapshot, &mut subscript_target, 16, 16, 8, 16);

    assert!(
        first_pixel_y(&subscript_target, 16, [255, 0, 0, 255])
            > first_pixel_y(&baseline_target, 16, [255, 0, 0, 255])
    );
}

#[test]
fn pixel_renderer_swaps_foreground_and_background_for_inverse_cells() {
    let mut grid = TerminalGrid::new(TerminalSize::new(1, 1));
    let mut cell = Cell::with_char('A');
    cell.foreground = Color::Rgb(255, 0, 0);
    cell.background = Color::Rgb(0, 0, 255);
    cell.inverse = true;
    grid.set(0, 0, cell);
    let snapshot = TerminalRenderSnapshot::from_grid(&grid);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert!(
        target
            .chunks_exact(4)
            .any(|pixel| pixel == [255, 0, 0, 255]),
        "renderer did not use the original foreground as inverse background"
    );
    assert!(
        target
            .chunks_exact(4)
            .any(|pixel| pixel == [0, 0, 255, 255]),
        "renderer did not use the original background as inverse foreground"
    );
}

#[test]
fn render_snapshot_exposes_terminal_cursor() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"ab\r\nc");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    let cursor = snapshot.cursor().expect("cursor should be visible");
    assert_eq!(cursor.row, 1);
    assert_eq!(cursor.column, 1);
    assert!(!cursor.blinking);
}

#[test]
fn render_snapshot_marks_blinking_terminal_cursor() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?12h");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert!(
        snapshot
            .cursor()
            .expect("cursor should be visible")
            .blinking
    );
}

#[test]
fn pixel_renderer_hides_blinking_cursor_when_phase_is_hidden() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?12h");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::with_blink_visible(false);
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert!(
        !target
            .chunks_exact(4)
            .any(|pixel| pixel == [229, 229, 229, 255]),
        "renderer drew a cursor during the hidden blink phase"
    );
}

#[test]
fn pixel_renderer_applies_blinking_cursor_opacity() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?12h");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::with_cursor_opacity(0.5);
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(&target[0..4], &[120, 120, 120, 255]);
}

#[test]
fn pixel_renderer_cursor_opacity_preserves_animation_frame() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    feed_red_green_inline_gif(&mut terminal, "width=1;height=1");
    terminal.feed(b"\r\x1b[?25h\x1b[?12h");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut renderer = PixelRenderer::with_animation_frame(1);
    renderer.set_cursor_opacity(0.5);
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(&target[0..4], &[114, 242, 114, 255]);
}

#[test]
fn render_snapshot_can_show_scrollback_viewport() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"ab\r\ncd\r\nef");

    let snapshot = TerminalRenderSnapshot::from_terminal_viewport(&terminal, 1);

    assert_eq!(
        snapshot
            .cells()
            .iter()
            .map(|cell| (cell.row, cell.column, cell.ch))
            .collect::<Vec<_>>(),
        vec![(0, 0, 'a'), (0, 1, 'b'), (1, 0, 'c'), (1, 1, 'd')]
    );
    assert!(snapshot.cursor().is_none());
}

#[test]
fn render_snapshot_omits_hidden_terminal_cursor() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"\x1b[?25l");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    assert!(snapshot.cursor().is_none());
}

#[test]
fn pixel_renderer_draws_blank_cursor_cell() {
    let terminal = Terminal::new(TerminalSize::new(1, 1));
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert!(
        target
            .chunks_exact(4)
            .any(|pixel| pixel == [229, 229, 229, 255]),
        "renderer did not draw a visible cursor block"
    );
}

#[test]
fn pixel_renderer_draws_configured_block_cursor_border() {
    let terminal = Terminal::new(TerminalSize::new(1, 1));
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut renderer = PixelRenderer::new();
    renderer.set_default_cursor_color([7, 8, 9, 255]);
    renderer.set_default_cursor_border(Some([1, 2, 3, 255]));
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [1, 2, 3, 255]);
    assert_eq!(pixel_at(&target, 8, 1, 1), [7, 8, 9, 255]);
}

#[test]
fn pixel_renderer_draws_bar_cursor_shape() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[6 q");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.cursor().unwrap().shape, CursorShape::Bar);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [229, 229, 229, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 0), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_draws_underline_cursor_shape() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[4 q");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(snapshot.cursor().unwrap().shape, CursorShape::Underline);
    let renderer = PixelRenderer::new();
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 7), [229, 229, 229, 255]);
    assert_eq!(pixel_at(&target, 8, 0, 0), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_applies_cursor_thickness_override_to_line_cursors() {
    let mut bar_terminal = Terminal::new(TerminalSize::new(1, 1));
    bar_terminal.feed(b"\x1b[6 q");
    let bar_snapshot = TerminalRenderSnapshot::from_terminal(&bar_terminal);
    let renderer = PixelRenderer::with_cursor_thickness_px(3);
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&bar_snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 2, 0), [229, 229, 229, 255]);
    assert_eq!(pixel_at(&target, 8, 3, 0), [12, 12, 12, 255]);

    let mut underline_terminal = Terminal::new(TerminalSize::new(1, 1));
    underline_terminal.feed(b"\x1b[4 q");
    let underline_snapshot = TerminalRenderSnapshot::from_terminal(&underline_terminal);
    target.fill(0);

    renderer.render(&underline_snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 5), [229, 229, 229, 255]);
    assert_eq!(pixel_at(&target, 8, 0, 4), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_applies_cursor_thickness_percent_to_line_cursors() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[4 q");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::with_cursor_thickness_percent(200);
    let mut target = vec![0; 8 * 12 * 4];

    renderer.render(&snapshot, &mut target, 8, 12, 8, 12);

    assert_eq!(pixel_at(&target, 8, 0, 8), [229, 229, 229, 255]);
    assert_eq!(pixel_at(&target, 8, 0, 7), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_applies_cursor_thickness_cell_fraction_to_line_cursors() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[6 q");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::with_cursor_thickness_cell_fraction_per_mille(250);
    let mut target = vec![0; 8 * 12 * 4];

    renderer.render(&snapshot, &mut target, 8, 12, 8, 12);

    assert_eq!(pixel_at(&target, 8, 2, 0), [229, 229, 229, 255]);
    assert_eq!(pixel_at(&target, 8, 3, 0), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_applies_cursor_thickness_points_to_line_cursors() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[6 q");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::with_cursor_thickness_points(2);
    let mut target = vec![0; 8 * 12 * 4];

    renderer.render(&snapshot, &mut target, 8, 12, 8, 12);

    assert_eq!(pixel_at(&target, 8, 2, 0), [229, 229, 229, 255]);
    assert_eq!(pixel_at(&target, 8, 3, 0), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_scales_cursor_thickness_points_by_window_dpi() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[6 q");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut renderer = PixelRenderer::with_cursor_thickness_points(2);
    renderer.set_window_dpi(144);
    let mut target = vec![0; 8 * 12 * 4];

    renderer.render(&snapshot, &mut target, 8, 12, 8, 12);

    assert_eq!(pixel_at(&target, 8, 3, 0), [229, 229, 229, 255]);
    assert_eq!(pixel_at(&target, 8, 4, 0), [12, 12, 12, 255]);
}

#[test]
fn pixel_renderer_force_reverse_video_cursor_uses_cell_foreground() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[38;2;255;0;0;48;2;0;0;255mA\x1b[1;1H");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let renderer = PixelRenderer::with_force_reverse_video_cursor(true);
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [255, 0, 0, 255]);
}

#[test]
fn pixel_renderer_reverse_video_cursor_min_contrast_uses_default_cursor_colors() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[38;2;17;17;17;48;2;16;16;16mA\x1b[1;1H");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut renderer = PixelRenderer::with_force_reverse_video_cursor(true)
        .with_reverse_video_cursor_min_contrast(2.5);
    renderer.set_default_cursor_color([7, 8, 9, 255]);
    renderer.set_default_cursor_foreground(Some([1, 2, 3, 255]));
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert!(count_pixels(&target, [1, 2, 3, 255]) > 0);
    assert!(count_pixels(&target, [7, 8, 9, 255]) > 0);
}

#[test]
fn pixel_renderer_cursor_color_overrides_force_reverse_video_cursor() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[38;2;255;0;0;48;2;0;0;255mA\x1b[1;1H");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal)
        .with_cursor_color(Some(Color::Rgb(0, 255, 0)));
    let renderer = PixelRenderer::with_force_reverse_video_cursor(true);
    let mut target = vec![0; 8 * 8 * 4];

    renderer.render(&snapshot, &mut target, 8, 8, 8, 8);

    assert_eq!(pixel_at(&target, 8, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&target, 8, 7, 7), [0, 255, 0, 255]);
}

fn snapshot_char(snapshot: &TerminalRenderSnapshot, row: u16, column: u16) -> Option<char> {
    snapshot
        .cells()
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
        .map(|cell| cell.ch)
}

fn feed_red_inline_png(terminal: &mut Terminal, params: &str) {
    const RED_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    feed_inline_image(terminal, params, RED_PNG_BASE64);
}

fn feed_red_inline_jpeg(terminal: &mut Terminal, params: &str) {
    const RED_JPEG_BASE64: &str = "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQH/2wBDAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQH/wAARCAABAAEDAREAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwD8X6/ynP8Av4P/2Q==";
    feed_inline_image(terminal, params, RED_JPEG_BASE64);
}

fn feed_red_inline_gif(terminal: &mut Terminal, params: &str) {
    const RED_GIF_BASE64: &str = "R0lGODdhAQABAIEAAP8AAAAAAAAAAAAAACwAAAAAAQABAAAIBAABBAQAOw==";
    feed_inline_image(terminal, params, RED_GIF_BASE64);
}

fn feed_red_green_inline_gif(terminal: &mut Terminal, params: &str) {
    const RED_GREEN_GIF_BASE64: &str = "R0lGODlhAQABAIEAAP8AAAAAAAAAAAAAACH/C05FVFNDQVBFMi4wAwEAAAAh+QQICgAAACwAAAAAAQABAAAIBAABBAQAIfkECAoAAAAsAAAAAAEAAQCBAP8AAAAAAAAAAAAACAQAAQQEADs=";
    feed_inline_image(terminal, params, RED_GREEN_GIF_BASE64);
}

fn red_green_gif_bytes() -> &'static [u8] {
    &[
        0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x81, 0x00, 0x00, 0xff, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0xff, 0x0b, 0x4e, 0x45,
        0x54, 0x53, 0x43, 0x41, 0x50, 0x45, 0x32, 0x2e, 0x30, 0x03, 0x01, 0x00, 0x00, 0x00, 0x21,
        0xf9, 0x04, 0x08, 0x0a, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
        0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x04, 0x04, 0x00, 0x21, 0xf9, 0x04, 0x08, 0x0a, 0x00,
        0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x81, 0x00, 0xff, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x04, 0x04,
        0x00, 0x3b,
    ]
}

fn red_green_blue_vertical_png_bytes() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x08, 0x06, 0x00, 0x00, 0x00, 0x52,
        0xdd, 0x65, 0x82, 0x00, 0x00, 0x00, 0x14, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0x00, 0x46, 0xff, 0x19, 0x18, 0x18, 0xfe, 0xff, 0x07, 0x00, 0x29, 0xe5, 0x05,
        0xfb, 0x48, 0xb8, 0xae, 0x8a, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42,
        0x60, 0x82,
    ]
}

fn feed_red_kitty_rgb_image(terminal: &mut Terminal) {
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=T,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
}

fn feed_compressed_red_kitty_rgb_image(terminal: &mut Terminal) {
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=T,f=24,s=1,v=1,c=1,r=1,o=z;eJz7z8AAAAMAAQA=\x1b\\");
}

fn feed_chunked_red_green_kitty_rgb_image(terminal: &mut Terminal) {
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=T,f=24,s=2,v=1,c=1,r=1,m=1;/wAA\x1b\\");
    terminal.feed(b"\x1b_Gm=0;AP8A\x1b\\");
}

fn feed_kitty_rgb_file_image(terminal: &mut Terminal, path: &Path, extra_params: &str) {
    feed_kitty_rgb_local_file_image(terminal, path, 'f', extra_params);
}

fn feed_kitty_rgb_temporary_file_image(terminal: &mut Terminal, path: &Path, extra_params: &str) {
    feed_kitty_rgb_local_file_image(terminal, path, 't', extra_params);
}

fn feed_kitty_rgb_local_file_image(
    terminal: &mut Terminal,
    path: &Path,
    medium: char,
    extra_params: &str,
) {
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    let encoded_path = base64_standard(path.as_os_str().to_string_lossy().as_bytes());
    let sequence =
        format!("\x1b_Ga=T,t={medium},f=24,s=1,v=1,c=1,r=1{extra_params};{encoded_path}\x1b\\");
    terminal.feed(sequence.as_bytes());
}

fn feed_stored_red_kitty_rgb_image(terminal: &mut Terminal) {
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
    terminal.feed(b"\x1b_Ga=p,i=7\x1b\\");
}

fn feed_overlapping_kitty_rgb_images(
    terminal: &mut Terminal,
    red_z_index: i32,
    green_z_index: i32,
) {
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
    terminal.feed(b"\x1b_Ga=t,i=8,f=24,s=1,v=1,c=1,r=1;AP8A\x1b\\");
    terminal.feed(format!("\x1b_Ga=p,i=7,z={red_z_index}\x1b\\").as_bytes());
    terminal.feed(b"\x1b[1;1H");
    terminal.feed(format!("\x1b_Ga=p,i=8,z={green_z_index}\x1b\\").as_bytes());
}

fn feed_overlapping_kitty_rgb_images_high_id_first(terminal: &mut Terminal) {
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
    terminal.feed(b"\x1b_Ga=t,i=8,f=24,s=1,v=1,c=1,r=1;AP8A\x1b\\");
    terminal.feed(b"\x1b_Ga=p,i=8,z=2\x1b\\");
    terminal.feed(b"\x1b[1;1H");
    terminal.feed(b"\x1b_Ga=p,i=7,z=2\x1b\\");
}

fn feed_inline_image(terminal: &mut Terminal, params: &str, payload_base64: &str) {
    terminal.feed(b"\x1b[?25l");
    terminal.take_damage();
    let sequence = format!("\x1b]1337;File=inline=1;{params}:{payload_base64}\x07");
    terminal.feed(sequence.as_bytes());
}

struct KittyTestFile {
    path: PathBuf,
}

impl KittyTestFile {
    fn new(data: &[u8]) -> Self {
        Self::new_with_prefix("rssh-kitty-file", data)
    }

    fn new_with_prefix(prefix: &str, data: &[u8]) -> Self {
        static NEXT_TEST_FILE_ID: AtomicUsize = AtomicUsize::new(0);

        let suffix = NEXT_TEST_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!("{prefix}-{}-{suffix}.rgb", std::process::id()));
        fs::write(&path, data).expect("write kitty test image file");
        Self { path }
    }
}

impl Drop for KittyTestFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn base64_standard(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut encoded = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let first = usize::from(chunk[0]);
        let second = usize::from(*chunk.get(1).unwrap_or(&0));
        let third = usize::from(*chunk.get(2).unwrap_or(&0));

        encoded.push(char::from(TABLE[first >> 2]));
        encoded.push(char::from(
            TABLE[((first & 0b0000_0011) << 4) | (second >> 4)],
        ));

        if chunk.len() > 1 {
            encoded.push(char::from(
                TABLE[((second & 0b0000_1111) << 2) | (third >> 6)],
            ));
        } else {
            encoded.push('=');
        }

        if chunk.len() > 2 {
            encoded.push(char::from(TABLE[third & 0b0011_1111]));
        } else {
            encoded.push('=');
        }
    }

    encoded
}

fn pixel_at(target: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let index = (y * width + x) * 4;
    [
        target[index],
        target[index + 1],
        target[index + 2],
        target[index + 3],
    ]
}

fn count_pixels(target: &[u8], color: [u8; 4]) -> usize {
    target
        .chunks_exact(4)
        .filter(|pixel| *pixel == color)
        .count()
}

fn first_pixel_y(target: &[u8], width: usize, color: [u8; 4]) -> usize {
    target
        .chunks_exact(4)
        .position(|pixel| pixel == color)
        .map(|index| index / width)
        .expect("expected color pixel")
}

#[test]
#[cfg(feature = "image-basic")]
fn image_basic_decodes_bmp_without_clipboard_dependency_features() {
    // A complete 1x1 red 24-bit BMP; no encoder features or GUI dependencies.
    let bmp = [
        0x42, 0x4d, 0x3a, 0, 0, 0, 0, 0, 0, 0, 0x36, 0, 0, 0, 0x28, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0,
        0, 1, 0, 0x18, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0x13, 0x0b, 0, 0, 0x13, 0x0b, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0xff, 0,
    ];
    let decoded = super::decode_image_rgba(&bmp, 0, None).expect("image-basic includes BMP");
    assert_eq!((decoded.width, decoded.height), (1, 1));
    assert_eq!(&*decoded.pixels, &[255, 0, 0, 255]);
}
