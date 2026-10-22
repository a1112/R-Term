use rterm_native::{
    CommandIntent, WindowEffect, WindowIntent, WindowState,
    persistence::{PersistenceCommand, PersistenceEffect},
    reduce,
};

#[test]
fn persistence_commands_emit_typed_port_effects_without_touching_the_filesystem() {
    let mut state = WindowState::default();
    let mut effects = Vec::new();

    reduce(
        &mut state,
        WindowIntent::Command(CommandIntent::Persistence(PersistenceCommand::Save)),
        &mut effects,
    );

    assert_eq!(
        effects,
        [WindowEffect::Persistence(PersistenceEffect::Save)]
    );
}
