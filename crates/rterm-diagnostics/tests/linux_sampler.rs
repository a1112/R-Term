use rterm_diagnostics::{LinuxPssSampler, MemoryMetric, SamplerError, parse_linux_smaps_rollup};

#[cfg(target_os = "linux")]
use rterm_diagnostics::MemorySampler;

#[test]
fn parses_pss_kib_and_checks_byte_conversion() {
    let input = concat!(
        "00400000-00452000 r-xp 00000000 00:00 0\n",
        "Rss:                 200 kB\n",
        "Pss:                 123 kB\n",
        "Pss_Dirty:            10 kB\n",
    );

    assert_eq!(parse_linux_smaps_rollup(input).unwrap(), 123 * 1024);
}

#[test]
fn pss_parser_rejects_rss_fallback_missing_duplicate_and_bad_units() {
    assert_eq!(
        parse_linux_smaps_rollup("Rss: 9 kB\n"),
        Err(SamplerError::MalformedResponse {
            metric: MemoryMetric::LinuxPssBytes,
            detail: "missing Pss field".to_owned(),
        })
    );
    assert!(matches!(
        parse_linux_smaps_rollup("Pss: 1 kB\nPss: 2 kB\n"),
        Err(SamplerError::MalformedResponse { detail, .. }) if detail.contains("duplicate")
    ));
    assert!(matches!(
        parse_linux_smaps_rollup("Pss: 1 MB\n"),
        Err(SamplerError::MalformedResponse { detail, .. }) if detail.contains("unit")
    ));
    assert!(matches!(
        parse_linux_smaps_rollup("Pss: nope kB\n"),
        Err(SamplerError::MalformedResponse { detail, .. }) if detail.contains("value")
    ));
}

#[test]
fn pss_parser_rejects_byte_overflow() {
    assert_eq!(
        parse_linux_smaps_rollup("Pss: 18014398509481984 kB\n"),
        Err(SamplerError::Overflow {
            metric: MemoryMetric::LinuxPssBytes,
        })
    );
}

#[test]
fn linux_sampler_always_declares_pss_as_its_metric() {
    assert_eq!(LinuxPssSampler::metric_kind(), MemoryMetric::LinuxPssBytes);
}

#[cfg(not(target_os = "linux"))]
#[test]
fn linux_sampler_is_explicitly_unsupported_off_linux() {
    assert!(matches!(
        LinuxPssSampler::new(std::process::id()),
        Err(SamplerError::Unsupported {
            metric: MemoryMetric::LinuxPssBytes,
            ..
        })
    ));
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "native live-child sampler probe"]
fn live_child_reports_nonzero_pss() {
    let mut sampler = LinuxPssSampler::new(std::process::id()).unwrap();

    assert_eq!(sampler.metric(), MemoryMetric::LinuxPssBytes);
    assert!(sampler.sample().unwrap() > 0);
}
