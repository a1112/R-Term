type Fixture = (&'static str, fn());

const FIXTURES: &[Fixture] = &[
    ("terminal_queries_frames_every_existing_fixed_and_dynamic_family", super::terminal_queries_frames_every_existing_fixed_and_dynamic_family),
    ("terminal_queries_frames_all_48_fixed_query_forms", super::terminal_queries_frames_all_48_fixed_query_forms),
    ("terminal_queries_semantically_classifies_all_48_fixed_queries", super::terminal_queries_semantically_classifies_all_48_fixed_queries),
    ("clipboard_payload_decode_enforces_the_one_megabyte_limit", super::clipboard_payload_decode_enforces_the_one_megabyte_limit),
    ("terminal_queries_semantically_classifies_every_dynamic_query_family", super::terminal_queries_semantically_classifies_every_dynamic_query_family),
    ("terminal_queries_rejects_malformed_mode_sequences_and_reserves_clipboard_controls", super::terminal_queries_rejects_malformed_mode_sequences_and_reserves_clipboard_controls),
    ("terminal_queries_distinguishes_key_modifier_query_from_reset_sequence", super::terminal_queries_distinguishes_key_modifier_query_from_reset_sequence),
    ("terminal_queries_fail_closed_on_oversized_reserved_queries_and_recovers", super::terminal_queries_fail_closed_on_oversized_reserved_queries_and_recovers),
    ("terminal_queries_resynchronizes_new_controls_inside_incomplete_csi", super::terminal_queries_resynchronizes_new_controls_inside_incomplete_csi),
    ("terminal_queries_can_and_sub_cancel_all_control_strings_before_following_queries", super::terminal_queries_can_and_sub_cancel_all_control_strings_before_following_queries),
    ("terminal_queries_uses_the_second_escape_for_overlapping_st", super::terminal_queries_uses_the_second_escape_for_overlapping_st),
    ("terminal_queries_reprocesses_invalid_utf8_successors_as_controls", super::terminal_queries_reprocesses_invalid_utf8_successors_as_controls),
    ("terminal_queries_reprocesses_strict_utf8_range_violations_as_controls", super::terminal_queries_reprocesses_strict_utf8_range_violations_as_controls),
    ("terminal_queries_holds_only_a_genuine_incomplete_utf8_suffix", super::terminal_queries_holds_only_a_genuine_incomplete_utf8_suffix),
    ("terminal_queries_discards_oversized_controls_and_recovers", super::terminal_queries_discards_oversized_controls_and_recovers),
    ("terminal_queries_reprocesses_escape_after_invalid_utf8_c1_in_discard_mode", super::terminal_queries_reprocesses_escape_after_invalid_utf8_c1_in_discard_mode),
    ("terminal_queries_keeps_discarding_csi_after_ordinary_utf8_c2_sequence", super::terminal_queries_keeps_discarding_csi_after_ordinary_utf8_c2_sequence),
    ("terminal_queries_preserves_every_byte_boundary_split", super::terminal_queries_preserves_every_byte_boundary_split),
    ("terminal_queries_frames_multiple_queries_in_one_chunk", super::terminal_queries_frames_multiple_queries_in_one_chunk),
    ("terminal_queries_finds_valid_query_after_unknown_same_family_control", super::terminal_queries_finds_valid_query_after_unknown_same_family_control),
    ("terminal_queries_cancels_strings_when_escape_does_not_form_st", super::terminal_queries_cancels_strings_when_escape_does_not_form_st),
    ("terminal_queries_does_not_treat_utf8_continuations_as_raw_c1", super::terminal_queries_does_not_treat_utf8_continuations_as_raw_c1),
    ("terminal_queries_releases_c2_when_the_next_byte_is_not_c1", super::terminal_queries_releases_c2_when_the_next_byte_is_not_c1),
    ("terminal_queries_frames_standalone_string_terminators", super::terminal_queries_frames_standalone_string_terminators),
    ("terminal_queries_passes_unknown_csi_osc_and_dcs_without_loss", super::terminal_queries_passes_unknown_csi_osc_and_dcs_without_loss),
    ("terminal_queries_supports_raw_and_utf8_c1_forms", super::terminal_queries_supports_raw_and_utf8_c1_forms),
    ("terminal_queries_inspects_no_more_than_four_times_the_input", super::terminal_queries_inspects_no_more_than_four_times_the_input),
    ("terminal_queries_chunk_size_work_ratio_is_bounded", super::terminal_queries_chunk_size_work_ratio_is_bounded),
    ("terminal_queries_work_counter_is_disabled_by_default_and_saturates", super::terminal_queries_work_counter_is_disabled_by_default_and_saturates),
];

pub(super) fn replay(test_name: &str) -> bool {
    let Some((_, test)) = FIXTURES.iter().find(|(name, _)| *name == test_name) else {
        return false;
    };
    test();
    true
}
