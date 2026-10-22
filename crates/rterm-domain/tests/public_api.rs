use rterm_domain::{
    PaneId, TabId, WindowId, WorkspaceId,
    app_shell::{
        PaneLaunch, PaneLaunchDomain, SshAuthDescription, SshKnownHostsPolicy, SshPaneLaunch,
    },
    session::{SessionLifecycle, SessionState},
};
use rterm_types::SessionId;
use std::collections::HashSet;

#[test]
fn domain_identifiers_and_session_lifecycle_are_stable() {
    let mut panes = HashSet::new();
    panes.insert(PaneId::new(4));
    assert_eq!(WindowId::new(1).get(), 1);
    assert_eq!(WorkspaceId::new(2).get(), 2);
    assert_eq!(TabId::new(3).get(), 3);
    assert!(panes.contains(&PaneId::new(4)));
    let mut lifecycle = SessionLifecycle::new(SessionId::new(7));
    lifecycle.start_connecting().unwrap();
    lifecycle.mark_connected().unwrap();
    assert_eq!(lifecycle.state(), SessionState::Connected);
}

#[test]
fn ssh_child_launch_keeps_identity_but_drops_explicit_command() {
    let ssh = SshPaneLaunch::new(
        "example.test",
        SshAuthDescription::Agent,
        SshKnownHostsPolicy::Prompt,
    )
    .with_remote_command(["uname", "-a"]);
    let child = PaneLaunch::ssh(ssh).for_child_pane();
    let PaneLaunchDomain::Ssh(child_ssh) = child.domain() else {
        panic!("child launch lost its SSH domain");
    };
    assert_eq!(child_ssh.target(), "example.test");
    assert!(child_ssh.remote_command().is_empty());
}
