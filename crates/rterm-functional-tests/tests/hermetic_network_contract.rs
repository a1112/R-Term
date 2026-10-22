#[test]
fn application_children_receive_a_loopback_only_proxy_environment() {
    let runner = include_str!("../src/runner.rs");
    let transport = include_str!("../src/transport_driver.rs");
    assert!(runner.contains("hermetic_app_command"));
    assert!(runner.contains("apply_loopback_only_environment(&mut command)"));
    assert!(!runner.contains("Command::new(app)"));
    assert!(transport.contains("hermetic_app_command"));
    assert!(!transport.contains("Command::new(app)"));
}

#[test]
fn ci_rejects_external_network_endpoints_before_running_scenarios() {
    let workflow = include_str!("../../../docs/trial/legacy-workflows/functional.yml");
    assert!(workflow.contains("check-functional-hermeticity.py"));
    assert!(workflow.contains("test_check_functional_hermeticity"));
}

#[test]
fn browser_context_aborts_every_non_loopback_request() {
    let browser = include_str!("../../../web/tests/terminal.spec.ts");
    assert!(browser.contains("installLoopbackOnlyNetworkPolicy"));
    assert!(browser.contains("route.abort('blockedbyclient')"));
}
