use rterm_diagnostics::{
    DiagnosticAttributionStage, DiagnosticGpuBackend, MarkerCollector, MarkerDisposition,
    MarkerError, MarkerIdentity, MarkerKind, ProjectOwnedResourceMetricsV1, RendererKind, Scenario,
};

const FIRST_PRESENT: &str = concat!(
    "rterm_diagnostic ",
    r#"{"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"first_present","elapsed_ms":12,"renderer":"cpu"}"#
);

const GPU_READY: &str = concat!(
    "rterm_diagnostic ",
    r#"{"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"gpu_ready","elapsed_ms":50,"renderer":"gpu"}"#
);

fn collector() -> MarkerCollector {
    MarkerCollector::new(MarkerIdentity::new("r1", 42, Scenario::EmptyWindow))
}

#[test]
fn parser_ignores_plain_output_and_accepts_cpu_first_gpu_later() {
    let mut collector = collector();

    assert_eq!(
        collector.push_line("ordinary diagnostic").unwrap(),
        MarkerDisposition::Ignored
    );
    assert!(matches!(
        collector.push_line(FIRST_PRESENT).unwrap(),
        MarkerDisposition::Accepted(record) if record.kind == MarkerKind::FirstPresent
    ));
    assert!(matches!(
        collector.push_line(GPU_READY).unwrap(),
        MarkerDisposition::Accepted(record) if record.kind == MarkerKind::GpuReady
    ));

    let trace = collector.trace();
    assert_eq!(trace.milestones.first_present_ms, Some(12));
    assert_eq!(trace.milestones.gpu_ready_ms, Some(50));
    assert_eq!(trace.first_renderer, Some(RendererKind::Cpu));
    assert_eq!(trace.final_renderer, Some(RendererKind::Gpu));
}

#[test]
fn gpu_ready_collects_selected_backend_and_adapter_identity() {
    let gpu_ready = concat!(
        "rterm_diagnostic ",
        r#"{"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"gpu_ready","elapsed_ms":50,"renderer":"gpu","gpu_backend":"Dx12","gpu_adapter_name":"fixture-adapter","gpu_adapter_vendor_id":4318,"gpu_adapter_device_id":9860,"gpu_adapter_type":"discrete-gpu"}"#
    );
    let mut collector = collector();

    collector.push_line(gpu_ready).unwrap();

    let trace = collector.trace();
    assert_eq!(
        trace.gpu_backend,
        Some(rterm_diagnostics::DiagnosticGpuBackend::Dx12)
    );
    assert_eq!(trace.gpu_adapter_name.as_deref(), Some("fixture-adapter"));
    assert_eq!(trace.gpu_adapter_vendor_id, Some(4318));
    assert_eq!(trace.gpu_adapter_device_id, Some(9860));
    assert_eq!(trace.gpu_adapter_type.as_deref(), Some("discrete-gpu"));
}

#[test]
fn missing_or_malformed_gpu_identity_preserves_gpu_ready_marker_semantics() {
    let malformed_gpu_ready = concat!(
        "rterm_diagnostic ",
        r#"{"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"gpu_ready","elapsed_ms":50,"renderer":"gpu","gpu_backend":7,"gpu_adapter_name":false,"gpu_adapter_vendor_id":"4318","gpu_adapter_device_id":-1,"gpu_adapter_type":{"unexpected":true}}"#
    );
    let mut malformed = collector();

    malformed.push_line(malformed_gpu_ready).unwrap();

    let trace = malformed.trace();
    assert_eq!(trace.milestones.gpu_ready_ms, Some(50));
    assert_eq!(trace.final_renderer, Some(RendererKind::Gpu));
    assert_eq!(trace.gpu_backend, None);
    assert_eq!(trace.gpu_adapter_name, None);
    assert_eq!(trace.gpu_adapter_vendor_id, None);
    assert_eq!(trace.gpu_adapter_device_id, None);
    assert_eq!(trace.gpu_adapter_type, None);

    let mut legacy = collector();
    legacy.push_line(GPU_READY).unwrap();
    let trace = legacy.trace();
    assert_eq!(trace.milestones.gpu_ready_ms, Some(50));
    assert_eq!(trace.final_renderer, Some(RendererKind::Gpu));
    assert_eq!(trace.gpu_backend, None);
    assert_eq!(trace.gpu_adapter_name, None);
    assert_eq!(trace.gpu_adapter_vendor_id, None);
    assert_eq!(trace.gpu_adapter_device_id, None);
    assert_eq!(trace.gpu_adapter_type, None);
}

#[test]
fn unknown_backend_and_overflowing_adapter_ids_are_ignored_independently() {
    let gpu_ready = concat!(
        "rterm_diagnostic ",
        r#"{"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"gpu_ready","elapsed_ms":50,"renderer":"gpu","gpu_backend":"future-backend","gpu_adapter_name":"fixture-adapter","gpu_adapter_vendor_id":4294967296,"gpu_adapter_device_id":4294967296,"gpu_adapter_type":"discrete-gpu"}"#
    );
    let mut collector = collector();

    collector.push_line(gpu_ready).unwrap();

    let trace = collector.trace();
    assert_eq!(trace.milestones.gpu_ready_ms, Some(50));
    assert_eq!(trace.final_renderer, Some(RendererKind::Gpu));
    assert_eq!(trace.gpu_backend, None);
    assert_eq!(trace.gpu_adapter_vendor_id, None);
    assert_eq!(trace.gpu_adapter_device_id, None);
    assert_eq!(trace.gpu_adapter_name.as_deref(), Some("fixture-adapter"));
    assert_eq!(trace.gpu_adapter_type.as_deref(), Some("discrete-gpu"));
}

#[test]
fn malformed_prefixed_json_is_not_treated_as_plain_output() {
    let error = collector()
        .push_line("rterm_diagnostic {broken")
        .unwrap_err();

    assert!(matches!(error, MarkerError::Malformed { .. }));
}

#[test]
fn marker_identity_mismatch_rejects_cross_run_and_cross_process_lines() {
    let wrong_run = FIRST_PRESENT.replace("\"r1\"", "\"r2\"");
    assert!(matches!(
        collector().push_line(&wrong_run),
        Err(MarkerError::IdentityMismatch {
            field: "run_id",
            ..
        })
    ));

    let wrong_pid = FIRST_PRESENT.replace("\"pid\":42", "\"pid\":43");
    assert!(matches!(
        collector().push_line(&wrong_pid),
        Err(MarkerError::IdentityMismatch { field: "pid", .. })
    ));

    let wrong_scenario = FIRST_PRESENT.replace("empty_window", "ssh1");
    assert!(matches!(
        collector().push_line(&wrong_scenario),
        Err(MarkerError::IdentityMismatch {
            field: "scenario",
            ..
        })
    ));
}

#[test]
fn marker_timestamps_must_not_decrease() {
    let mut collector = collector();
    collector.push_line(GPU_READY).unwrap();

    let error = collector.push_line(FIRST_PRESENT).unwrap_err();
    assert_eq!(
        error,
        MarkerError::DecreasingElapsed {
            previous_ms: 50,
            observed_ms: 12,
        }
    );
}

#[test]
fn singleton_and_terminal_markers_reject_duplicates() {
    let mut collector = collector();
    collector.push_line(FIRST_PRESENT).unwrap();
    assert_eq!(
        collector.push_line(FIRST_PRESENT).unwrap_err(),
        MarkerError::Duplicate(MarkerKind::FirstPresent)
    );

    let exited = concat!(
        "rterm_diagnostic ",
        r#"{"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"process_exited","elapsed_ms":60}"#
    );
    collector.push_line(exited).unwrap();
    assert_eq!(
        collector.push_line(exited).unwrap_err(),
        MarkerError::Duplicate(MarkerKind::ProcessExited)
    );
}

#[test]
fn selected_backend_marker_fields_are_preserved_for_result_enrichment() {
    let line = FIRST_PRESENT.replace(
        "\"renderer\":\"cpu\"",
        "\"renderer\":\"cpu\",\"backend\":\"dx12\",\"adapter_name\":\"fixture-adapter\",\"future_detail\":{\"value\":7}",
    );
    let disposition = collector().push_line(&line).unwrap();
    let MarkerDisposition::Accepted(record) = disposition else {
        panic!("marker was not accepted");
    };

    assert_eq!(record.extra["backend"], "dx12");
    assert_eq!(record.extra["adapter_name"], "fixture-adapter");
    assert_eq!(record.extra["future_detail"]["value"], 7);
}

#[test]
fn unknown_schema_and_marker_kind_are_protocol_errors() {
    let wrong_schema = FIRST_PRESENT.replace("rterm.diagnostics/v2", "rterm.diagnostics/v3");
    assert!(matches!(
        collector().push_line(&wrong_schema),
        Err(MarkerError::Malformed { .. })
    ));

    let wrong_kind = FIRST_PRESENT.replace("first_present", "future_kind");
    assert!(matches!(
        collector().push_line(&wrong_kind),
        Err(MarkerError::Malformed { .. })
    ));
}

#[test]
fn attribution_stage_ready_is_a_typed_singleton_protocol_marker() {
    let resources = ProjectOwnedResourceMetricsV1 {
        cpu_staging_bytes: 4,
        cpu_surface_count: 1,
        cpu_present_count: 1,
        ..ProjectOwnedResourceMetricsV1::default()
    };
    let ready = serde_json::json!({
        "schema": "rterm.diagnostics/v2",
        "run_id": "r1",
        "pid": 42,
        "scenario": "empty_window",
        "kind": "attribution_stage_ready",
        "elapsed_ms": 50,
        "renderer": "cpu",
        "requested_stage": "cpu-window",
        "final_stage": "cpu-window",
        "resource_summary_schema": "rterm.project-owned-resources/v1",
        "resource_summary": &resources,
    });
    let line = format!("rterm_diagnostic {ready}");
    let mut collector = MarkerCollector::new_attribution(
        MarkerIdentity::new("r1", 42, Scenario::EmptyWindow),
        DiagnosticAttributionStage::CpuWindow,
    );
    for line in [
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"process_started","elapsed_ms":0}"#,
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"window_created","elapsed_ms":1}"#,
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"first_present","elapsed_ms":2,"renderer":"cpu"}"#,
    ] {
        collector.push_line(line).unwrap();
    }

    let MarkerDisposition::Accepted(record) = collector
        .push_line(&line)
        .expect("typed attribution_stage_ready marker")
    else {
        panic!("attribution stage marker was ignored");
    };
    assert_eq!(
        serde_json::to_value(record.kind).unwrap(),
        "attribution_stage_ready"
    );

    assert_eq!(
        collector.trace().final_attribution_stage,
        Some(DiagnosticAttributionStage::CpuWindow)
    );
    assert_eq!(
        collector.trace().resource_summary.as_ref(),
        Some(&resources)
    );

    let duplicate = collector.push_line(&line).unwrap_err();
    assert!(
        duplicate.to_string().contains("duplicate marker"),
        "singleton marker failed with the wrong error: {duplicate}"
    );
}

#[test]
fn attribution_adapter_stage_carries_identity_without_a_forbidden_gpu_ready_marker() {
    let resources = ProjectOwnedResourceMetricsV1 {
        cpu_staging_bytes: 4,
        cpu_surface_count: 1,
        cpu_present_count: 1,
        instance_count: 1,
        surface_count: 1,
        adapter_count: 1,
        device_count: 1,
        queue_count: 1,
        backend: Some(DiagnosticGpuBackend::Dx12),
        adapter_name: Some("fixture-adapter".to_owned()),
        ..ProjectOwnedResourceMetricsV1::default()
    };
    let ready = serde_json::json!({
        "schema": "rterm.diagnostics/v2",
        "run_id": "r1",
        "pid": 42,
        "scenario": "empty_window",
        "kind": "attribution_stage_ready",
        "elapsed_ms": 50,
        "renderer": "cpu",
        "requested_stage": "adapter-device",
        "final_stage": "adapter-device",
        "resource_summary_schema": "rterm.project-owned-resources/v1",
        "resource_summary": &resources,
        "gpu_adapter_vendor_id": 4318,
        "gpu_adapter_device_id": 9860,
        "gpu_adapter_type": "discrete-gpu",
    });
    let mut collector = MarkerCollector::new_attribution(
        MarkerIdentity::new("r1", 42, Scenario::EmptyWindow),
        DiagnosticAttributionStage::AdapterDevice,
    );
    for line in [
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"process_started","elapsed_ms":0}"#,
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"window_created","elapsed_ms":1}"#,
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"first_present","elapsed_ms":2,"renderer":"cpu"}"#,
    ] {
        collector.push_line(line).unwrap();
    }

    collector
        .push_line(&format!("rterm_diagnostic {ready}"))
        .expect("adapter attribution marker carries its own identity");
    let trace = collector.trace();
    assert_eq!(trace.final_renderer, Some(RendererKind::Cpu));
    assert_eq!(trace.gpu_backend, Some(DiagnosticGpuBackend::Dx12));
    assert_eq!(trace.gpu_adapter_name.as_deref(), Some("fixture-adapter"));
    assert_eq!(trace.gpu_adapter_vendor_id, Some(4318));
    assert_eq!(trace.gpu_adapter_device_id, Some(9860));
    assert_eq!(trace.gpu_adapter_type.as_deref(), Some("discrete-gpu"));
}

#[test]
fn attribution_stage_protocol_rejects_order_later_activity_and_unknown_resources() {
    let identity = || MarkerIdentity::new("r1", 42, Scenario::EmptyWindow);
    let ready = |resources: serde_json::Value| {
        format!(
            "rterm_diagnostic {}",
            serde_json::json!({
                "schema": "rterm.diagnostics/v2",
                "run_id": "r1",
                "pid": 42,
                "scenario": "empty_window",
                "kind": "attribution_stage_ready",
                "elapsed_ms": 3,
                "renderer": "cpu",
                "requested_stage": "cpu-window",
                "final_stage": "cpu-window",
                "resource_summary_schema": "rterm.project-owned-resources/v1",
                "resource_summary": resources,
            })
        )
    };
    let valid_resources = serde_json::to_value(ProjectOwnedResourceMetricsV1 {
        cpu_staging_bytes: 4,
        cpu_surface_count: 1,
        cpu_present_count: 1,
        ..ProjectOwnedResourceMetricsV1::default()
    })
    .unwrap();

    let mut out_of_order =
        MarkerCollector::new_attribution(identity(), DiagnosticAttributionStage::CpuWindow);
    assert!(
        out_of_order
            .push_line(&ready(valid_resources.clone()))
            .is_err()
    );

    let mut unknown_resources = valid_resources.clone();
    unknown_resources["driver_private_bytes"] = serde_json::json!(1);
    let mut unknown =
        MarkerCollector::new_attribution(identity(), DiagnosticAttributionStage::CpuWindow);
    for line in [
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"process_started","elapsed_ms":0}"#,
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"window_created","elapsed_ms":1}"#,
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"first_present","elapsed_ms":2,"renderer":"cpu"}"#,
    ] {
        unknown.push_line(line).unwrap();
    }
    assert!(unknown.push_line(&ready(unknown_resources)).is_err());

    let mut later =
        MarkerCollector::new_attribution(identity(), DiagnosticAttributionStage::CpuWindow);
    for line in [
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"process_started","elapsed_ms":0}"#,
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"window_created","elapsed_ms":1}"#,
        r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"first_present","elapsed_ms":2,"renderer":"cpu"}"#,
    ] {
        later.push_line(line).unwrap();
    }
    later.push_line(&ready(valid_resources)).unwrap();
    let config_after_ready = r#"rterm_diagnostic {"schema":"rterm.diagnostics/v2","run_id":"r1","pid":42,"scenario":"empty_window","kind":"config_ready","elapsed_ms":4}"#;
    assert!(later.push_line(config_after_ready).is_err());
}
