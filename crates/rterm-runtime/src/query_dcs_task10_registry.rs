type Fixture = (&'static str, fn());

const FIXTURES: &[Fixture] = &[
    ("parses_decrqss_kinds_from_all_dcs_and_st_encodings", super::parses_decrqss_kinds_from_all_dcs_and_st_encodings),
    ("preserves_unknown_decrqss_request_for_failure_response", super::preserves_unknown_decrqss_request_for_failure_response),
    ("decrqss_parser_is_anchored_and_requires_exact_consumption", super::decrqss_parser_is_anchored_and_requires_exact_consumption),
    ("parses_xtgettcap_hex_names_and_preserves_terminator", super::parses_xtgettcap_hex_names_and_preserves_terminator),
    ("xtgettcap_hex_is_checked_without_losing_invalid_names", super::xtgettcap_hex_is_checked_without_losing_invalid_names),
    ("xtgettcap_parser_is_anchored_and_requires_exact_consumption", super::xtgettcap_parser_is_anchored_and_requires_exact_consumption),
    ("embedded_st_cannot_hide_trailing_data", super::embedded_st_cannot_hide_trailing_data),
    ("bounds_xtgettcap_entries_and_aggregate_bytes", super::bounds_xtgettcap_entries_and_aggregate_bytes),
    ("bounds_decrqss_content_and_keeps_it_reserved", super::bounds_decrqss_content_and_keeps_it_reserved),
];

pub(super) fn replay(test_name: &str) -> bool {
    let Some((_, test)) = FIXTURES.iter().find(|(name, _)| *name == test_name) else {
        return false;
    };
    test();
    true
}
