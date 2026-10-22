use rterm_domain::PaneId;
use rterm_native::{
    ClipboardEffect, PaneCommand, PaneLifecycleIntent, SpawnEffect, UriEffect, WindowEffect,
    WindowIntent, WindowState, commands::CommandIntent, reduce,
};
use rterm_runtime::{PaneToken, PaneTokenAllocator};

fn token(pane: u64) -> PaneToken {
    PaneTokenAllocator::new()
        .issue(PaneId::new(pane))
        .expect("pane generation")
}

fn apply(state: &mut WindowState, intent: WindowIntent) -> Vec<WindowEffect> {
    let mut effects = Vec::new();
    reduce(state, intent, &mut effects);
    effects
}

#[test]
fn parsed_commands_route_to_uri_clipboard_spawn_and_persistence_ports() {
    let mut state = WindowState::default();
    let pane = token(7);
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(pane)),
    );

    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Command(CommandIntent::OpenUri("https://example.test".to_owned()))
        ),
        [WindowEffect::Uri(UriEffect::Open(
            "https://example.test".to_owned()
        ))]
    );
    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Command(CommandIntent::Copy("copy".to_owned()))
        ),
        [WindowEffect::Clipboard(ClipboardEffect::Write {
            context: None,
            selection: None,
            contents: "copy".to_owned(),
        })]
    );
    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Command(CommandIntent::Pane(PaneCommand::Spawn)),
        ),
        [WindowEffect::Spawn(SpawnEffect::Pane)]
    );
}
