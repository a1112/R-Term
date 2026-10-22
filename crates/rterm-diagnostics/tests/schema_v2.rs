use rterm_diagnostics::{
    ConnectionState, DiagnosticAttributionStage, DiagnosticGpuBackend, DiagnosticRendererMode,
    DiagnosticsResult, MemoryMetric, Platform, ProjectOwnedResourceMetricsV1,
    ProjectOwnedResourceSchemaVersion, RendererKind, RunConfiguration, RunIdentity, Scenario,
    SchemaVersion, StartupMilestones,
};
use serde_json::json;

#[test]
fn diagnostic_renderer_mode_exposes_stable_cli_value() {
    assert_eq!(DiagnosticRendererMode::Auto.as_str(), "auto");
}

#[test]
fn diagnostic_gpu_backend_parses_supported_cli_value() {
    assert_eq!(
        "dx12".parse::<DiagnosticGpuBackend>().unwrap(),
        DiagnosticGpuBackend::Dx12
    );
}

#[test]
fn diagnostic_gpu_backend_rejects_unsupported_cli_value() {
    assert!("metal".parse::<DiagnosticGpuBackend>().is_err());
}

#[test]
fn schema_v2_serializes_stable_discriminators_and_optional_hybrid_milestones() {
    let result = DiagnosticsResult::successful_fixture(
        RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
        MemoryMetric::WindowsPrivateWorkingSetBytes,
        RunConfiguration::default(),
    );
    let value = serde_json::to_value(result).unwrap();

    assert_eq!(value["schema"], "rterm.diagnostics/v2");
    assert_eq!(value["run"]["scenario"], "empty_window");
    assert_eq!(
        value["memory"]["metric"],
        "windows_private_working_set_bytes"
    );
    assert!(value["milestones"]["gpu_ready_ms"].is_null());
    assert_eq!(value["renderer"]["first"], "cpu");
    assert_eq!(value["connection"]["final_state"], "not_started");
}

#[test]
fn schema_v2_default_configuration_preserves_the_legacy_wire_shape() {
    let result = DiagnosticsResult::successful_fixture(
        RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
        MemoryMetric::WindowsPrivateWorkingSetBytes,
        RunConfiguration::default(),
    );

    let value = serde_json::to_value(result).unwrap();

    assert_eq!(
        value["configuration"],
        json!({
            "stabilization_ms": 5_000,
            "sample_interval_ms": 100,
            "sample_count": 10,
            "columns": 80,
            "rows": 24,
            "scale_factor_milli": 1_000,
        })
    );
}

#[test]
fn schema_v2_serializes_an_explicit_renderer_without_a_backend() {
    let configuration = RunConfiguration {
        requested_renderer: DiagnosticRendererMode::Cpu,
        ..RunConfiguration::default()
    };
    let result = DiagnosticsResult::successful_fixture(
        RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
        MemoryMetric::WindowsPrivateWorkingSetBytes,
        configuration,
    );

    let value = serde_json::to_value(result).unwrap();

    assert_eq!(value["configuration"]["requested_renderer"], "cpu");
    assert!(
        value["configuration"]
            .as_object()
            .is_some_and(|configuration| !configuration.contains_key("requested_gpu_backend"))
    );
}

#[test]
fn schema_v2_accepts_cpu_first_and_gpu_ready_later() {
    let mut result = DiagnosticsResult::successful_fixture(
        RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
        MemoryMetric::WindowsPrivateWorkingSetBytes,
        RunConfiguration::default(),
    );
    result.milestones = StartupMilestones {
        process_started_ms: 0,
        first_present_ms: Some(10),
        gpu_ready_ms: Some(40),
        scenario_ready_ms: Some(10),
        sampling_started_ms: Some(5_010),
        sampling_finished_ms: Some(5_910),
        process_exited_ms: Some(6_000),
        ..StartupMilestones::default()
    };
    result.renderer.first = Some(RendererKind::Cpu);
    result.renderer.final_renderer = Some(RendererKind::Gpu);
    result.connection.final_state = ConnectionState::NotStarted;

    result.validate().unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let decoded: DiagnosticsResult = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.schema, SchemaVersion::V2);
    assert_eq!(decoded.milestones.first_present_ms, Some(10));
    assert_eq!(decoded.milestones.gpu_ready_ms, Some(40));
}

#[test]
fn schema_v2_decodes_legacy_results_without_backend_identity() {
    let result = DiagnosticsResult::successful_fixture(
        RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
        MemoryMetric::WindowsPrivateWorkingSetBytes,
        RunConfiguration::default(),
    );
    let mut legacy = serde_json::to_value(result).unwrap();
    let configuration = legacy["configuration"].as_object_mut().unwrap();
    configuration.remove("requested_renderer");
    configuration.remove("requested_gpu_backend");
    let renderer = legacy["renderer"].as_object_mut().unwrap();
    renderer.remove("backend");
    renderer.remove("adapter_name");
    renderer.remove("adapter_vendor_id");
    renderer.remove("adapter_device_id");
    renderer.remove("adapter_type");

    let decoded: DiagnosticsResult = serde_json::from_value(legacy).unwrap();

    assert_eq!(
        decoded.configuration.requested_renderer,
        DiagnosticRendererMode::Auto
    );
    assert_eq!(decoded.configuration.requested_gpu_backend, None);
    assert_eq!(decoded.renderer.backend, None);
    assert_eq!(decoded.renderer.adapter_name, None);
    assert_eq!(decoded.renderer.adapter_vendor_id, None);
    assert_eq!(decoded.renderer.adapter_device_id, None);
    assert_eq!(decoded.renderer.adapter_type, None);
}

#[test]
fn schema_v2_serializes_requested_and_selected_gpu_backend() {
    let configuration = RunConfiguration {
        requested_renderer: DiagnosticRendererMode::Auto,
        requested_gpu_backend: Some(DiagnosticGpuBackend::Dx12),
        ..RunConfiguration::default()
    };
    let mut result = DiagnosticsResult::successful_fixture(
        RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
        MemoryMetric::WindowsPrivateWorkingSetBytes,
        configuration,
    );
    result.renderer.backend = Some(DiagnosticGpuBackend::Dx12);
    result.renderer.adapter_name = Some("fixture-adapter".to_owned());
    result.renderer.adapter_vendor_id = Some(0x10de);
    result.renderer.adapter_device_id = Some(0x1234);
    result.renderer.adapter_type = Some("discrete-gpu".to_owned());

    let value = serde_json::to_value(result).unwrap();

    assert_eq!(value["configuration"]["requested_renderer"], "auto");
    assert_eq!(value["configuration"]["requested_gpu_backend"], "dx12");
    assert_eq!(value["renderer"]["backend"], "dx12");
    assert_eq!(value["renderer"]["adapter_name"], "fixture-adapter");
    assert_eq!(value["renderer"]["adapter_vendor_id"], 0x10de);
    assert_eq!(value["renderer"]["adapter_device_id"], 0x1234);
    assert_eq!(value["renderer"]["adapter_type"], "discrete-gpu");
}

#[test]
fn successful_schema_requires_the_configured_sample_count() {
    let mut result = DiagnosticsResult::successful_fixture(
        RunIdentity::fixture(Scenario::EmptyWindow, Platform::Linux),
        MemoryMetric::LinuxPssBytes,
        RunConfiguration::default(),
    );
    result.memory.samples.pop();

    let error = result.validate().unwrap_err();
    assert_eq!(error.to_string(), "expected 10 memory samples, observed 9");
}

#[test]
fn schema_version_rejects_unknown_wire_values() {
    let error = serde_json::from_str::<SchemaVersion>(r#""rterm.diagnostics/v3""#).unwrap_err();
    assert!(error.to_string().contains("rterm.diagnostics/v2"));
}

#[test]
fn attribution_stage_schema_uses_exact_kebab_case_and_closed_resource_discriminator() {
    for (wire, stage) in [
        ("cpu-window", DiagnosticAttributionStage::CpuWindow),
        (
            "instance-surface",
            DiagnosticAttributionStage::InstanceSurface,
        ),
        ("adapter-device", DiagnosticAttributionStage::AdapterDevice),
        (
            "configured-surface-clear",
            DiagnosticAttributionStage::ConfiguredSurfaceClear,
        ),
        (
            "layer-pipelines",
            DiagnosticAttributionStage::LayerPipelines,
        ),
        (
            "fixture-font-text",
            DiagnosticAttributionStage::FixtureFontText,
        ),
        (
            "platform-font-index",
            DiagnosticAttributionStage::PlatformFontIndex,
        ),
        ("full-frame", DiagnosticAttributionStage::FullFrame),
    ] {
        assert_eq!(wire.parse::<DiagnosticAttributionStage>(), Ok(stage));
        assert_eq!(stage.as_str(), wire);
        assert_eq!(serde_json::to_value(stage).unwrap(), wire);
    }
    assert!(
        "fixture_font_text"
            .parse::<DiagnosticAttributionStage>()
            .is_err()
    );
    assert!(
        serde_json::from_str::<ProjectOwnedResourceSchemaVersion>(
            r#""rterm.project-owned-resources/v2""#
        )
        .is_err()
    );
}

#[test]
fn attribution_stage_result_has_complete_closed_v1_resources_without_changing_legacy_json() {
    let legacy = DiagnosticsResult::successful_fixture(
        RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
        MemoryMetric::WindowsPrivateWorkingSetBytes,
        RunConfiguration::default(),
    );
    let legacy_json = serde_json::to_string(&legacy).unwrap();
    assert!(!legacy_json.contains("attribution"));
    assert!(!legacy_json.contains("resource_summary"));

    let resources = ProjectOwnedResourceMetricsV1 {
        cpu_staging_bytes: 4,
        cpu_surface_count: 1,
        cpu_present_count: 1,
        ..ProjectOwnedResourceMetricsV1::default()
    };
    resources
        .validate_at(DiagnosticAttributionStage::CpuWindow)
        .expect("exact cpu-window resource row");
    let resource_json = serde_json::to_value(&resources).unwrap();
    let fields = resource_json.as_object().expect("resource summary object");
    assert_eq!(fields.len(), 35, "all numeric v1 fields must be present");

    let mut unknown = resource_json;
    unknown["future_resource"] = json!(0);
    assert!(serde_json::from_value::<ProjectOwnedResourceMetricsV1>(unknown).is_err());

    let configuration = RunConfiguration {
        requested_attribution_stage: Some(DiagnosticAttributionStage::CpuWindow),
        ..RunConfiguration::default()
    };
    let mut result = DiagnosticsResult::successful_fixture(
        RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
        MemoryMetric::WindowsPrivateWorkingSetBytes,
        configuration,
    );
    result.final_attribution_stage = Some(DiagnosticAttributionStage::CpuWindow);
    result.resource_summary_schema = Some(ProjectOwnedResourceSchemaVersion::V1);
    result.resource_summary = Some(resources);
    result.validate().expect("complete attribution result");

    let value = serde_json::to_value(result).unwrap();
    assert_eq!(
        value["configuration"]["requested_renderer"], "auto",
        "an attribution record must bind the explicit renderer request"
    );
    assert_eq!(
        value["configuration"]["requested_attribution_stage"],
        "cpu-window"
    );
    assert_eq!(value["final_attribution_stage"], "cpu-window");
    assert_eq!(
        value["resource_summary_schema"],
        "rterm.project-owned-resources/v1"
    );
}

fn exact_attribution_resources(stage: DiagnosticAttributionStage) -> ProjectOwnedResourceMetricsV1 {
    let mut resources = ProjectOwnedResourceMetricsV1 {
        cpu_staging_bytes: 4,
        cpu_surface_count: 1,
        cpu_present_count: 1,
        ..ProjectOwnedResourceMetricsV1::default()
    };
    if stage >= DiagnosticAttributionStage::InstanceSurface {
        resources.instance_count = 1;
        resources.surface_count = 1;
    }
    if stage >= DiagnosticAttributionStage::AdapterDevice {
        resources.adapter_count = 1;
        resources.device_count = 1;
        resources.queue_count = 1;
        resources.backend = Some(DiagnosticGpuBackend::Dx12);
        resources.adapter_name = Some("fixture-adapter".to_owned());
    }
    if stage >= DiagnosticAttributionStage::ConfiguredSurfaceClear {
        resources.surface_configure_count = 1;
        resources.surface_acquire_count = if stage >= DiagnosticAttributionStage::FullFrame {
            3
        } else if stage >= DiagnosticAttributionStage::FixtureFontText {
            2
        } else {
            1
        };
        resources.clear_present_count = 1;
    }
    if stage >= DiagnosticAttributionStage::LayerPipelines {
        resources.pipeline_count = 1;
        resources.pipeline_layout_count = 1;
        resources.materialized_buffer_count = 1;
        resources.total_allocated_buffer_bytes = 8;
    }
    if stage >= DiagnosticAttributionStage::FixtureFontText {
        resources.retained_font_bytes = 1;
        resources.active_font_count = 1;
        resources.catalog_builds = 1;
        resources.catalog_generation = 1;
        resources.glyph_atlas_bytes = 1;
        resources.total_allocated_texture_bytes = 1;
        resources.base_text_renderer_materialization_count = 1;
    }
    if stage >= DiagnosticAttributionStage::PlatformFontIndex {
        resources.indexed_font_count = 2;
    }
    if stage >= DiagnosticAttributionStage::FullFrame {
        resources.snapshot_bytes = 1;
        resources.base_text_renderer_materialization_count = 2;
    }
    resources
}

#[test]
fn attribution_stage_resource_matrix_is_exact_and_fail_closed_for_all_eight_rows() {
    for stage in DiagnosticAttributionStage::ORDERED {
        let exact = exact_attribution_resources(stage);
        exact
            .validate_at(stage)
            .unwrap_or_else(|violations| panic!("{stage} rejected: {violations:?}"));

        let mut product_service = exact.clone();
        product_service.post_ready_task_count = 1;
        assert!(
            product_service.validate_at(stage).is_err(),
            "{stage} accepted a later product service counter"
        );

        let mut later_resource = exact.clone();
        match stage {
            DiagnosticAttributionStage::CpuWindow => later_resource.instance_count = 1,
            DiagnosticAttributionStage::InstanceSurface => later_resource.adapter_count = 1,
            DiagnosticAttributionStage::AdapterDevice => {
                later_resource.surface_configure_count = 1;
            }
            DiagnosticAttributionStage::ConfiguredSurfaceClear => later_resource.pipeline_count = 1,
            DiagnosticAttributionStage::LayerPipelines => later_resource.retained_font_bytes = 1,
            DiagnosticAttributionStage::FixtureFontText => later_resource.indexed_font_count = 1,
            DiagnosticAttributionStage::PlatformFontIndex => later_resource.snapshot_bytes = 1,
            DiagnosticAttributionStage::FullFrame => later_resource.config_load_count = 1,
        }
        assert!(
            later_resource.validate_at(stage).is_err(),
            "{stage} accepted a forbidden later-stage resource"
        );
    }
}

#[test]
fn full_frame_rejects_image_bytes_for_the_fixed_empty_graph() {
    let mut resources = exact_attribution_resources(DiagnosticAttributionStage::FullFrame);
    resources.image_texture_bytes = 1;
    resources.total_allocated_texture_bytes =
        resources.total_allocated_texture_bytes.saturating_add(1);

    assert!(
        resources
            .validate_at(DiagnosticAttributionStage::FullFrame)
            .expect_err("the fixed empty FullFrame graph cannot own image resources")
            .iter()
            .any(|violation| violation.contains("image_texture_bytes"))
    );
}

#[test]
fn attribution_stage_backend_and_adapter_identity_begin_exactly_at_adapter_device() {
    for stage in DiagnosticAttributionStage::ORDERED {
        let mut resources = exact_attribution_resources(stage);
        if stage < DiagnosticAttributionStage::AdapterDevice {
            resources.backend = Some(DiagnosticGpuBackend::Dx12);
            resources.adapter_name = Some("fabricated".to_owned());
            assert!(resources.validate_at(stage).is_err());
        } else {
            resources.backend = None;
            assert!(resources.validate_at(stage).is_err());
            let mut resources = exact_attribution_resources(stage);
            resources.adapter_name = None;
            assert!(resources.validate_at(stage).is_err());
        }
    }
}

#[test]
fn attribution_stage_resource_json_rejects_missing_and_unknown_fields() {
    let resources = exact_attribution_resources(DiagnosticAttributionStage::FullFrame);
    let mut missing = serde_json::to_value(&resources).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("base_text_renderer_materialization_count");
    assert!(serde_json::from_value::<ProjectOwnedResourceMetricsV1>(missing).is_err());

    let mut unknown = serde_json::to_value(resources).unwrap();
    unknown["driver_private_bytes"] = json!(1);
    assert!(serde_json::from_value::<ProjectOwnedResourceMetricsV1>(unknown).is_err());
}

#[test]
fn attribution_stage_result_optional_group_is_consistent_and_fail_closed() {
    let fixture = || {
        DiagnosticsResult::successful_fixture(
            RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
            MemoryMetric::WindowsPrivateWorkingSetBytes,
            RunConfiguration::default(),
        )
    };

    let mut unrequested = fixture();
    unrequested.final_attribution_stage = Some(DiagnosticAttributionStage::CpuWindow);
    assert!(unrequested.validate().is_err());

    let mut missing = fixture();
    missing.configuration.requested_attribution_stage = Some(DiagnosticAttributionStage::CpuWindow);
    assert!(missing.validate().is_err());

    let mut mismatched = fixture();
    mismatched.configuration.requested_attribution_stage =
        Some(DiagnosticAttributionStage::CpuWindow);
    mismatched.final_attribution_stage = Some(DiagnosticAttributionStage::InstanceSurface);
    mismatched.resource_summary_schema = Some(ProjectOwnedResourceSchemaVersion::V1);
    mismatched.resource_summary = Some(exact_attribution_resources(
        DiagnosticAttributionStage::InstanceSurface,
    ));
    assert!(mismatched.validate().is_err());

    let mut invalid = fixture();
    invalid.configuration.requested_attribution_stage = Some(DiagnosticAttributionStage::CpuWindow);
    invalid.final_attribution_stage = Some(DiagnosticAttributionStage::CpuWindow);
    invalid.resource_summary_schema = Some(ProjectOwnedResourceSchemaVersion::V1);
    invalid.resource_summary = Some(ProjectOwnedResourceMetricsV1::default());
    assert!(invalid.validate().is_err());

    let mut failed = fixture();
    failed.configuration.requested_attribution_stage = Some(DiagnosticAttributionStage::FullFrame);
    failed.failures.push(rterm_diagnostics::DiagnosticFailure {
        code: "owner_failed".to_owned(),
        phase: "attribution".to_owned(),
        message: "fixture owner failed".to_owned(),
        os_error_code: None,
        recoverable: false,
        context: None,
    });
    failed.memory.samples.clear();
    assert!(
        failed.validate().is_ok(),
        "a structured failed run may omit owner evidence that was never produced"
    );
}
