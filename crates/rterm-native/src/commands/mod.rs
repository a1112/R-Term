use rterm_domain::PaneId;

use crate::{
    ClipboardEffect, PaneCommand, PersistenceCommand, RuntimePortEffect, SpawnEffect, UriEffect,
    WindowEffect, WindowPortEffect, WindowState, panes, persistence,
};

/// Parsed user or automation commands accepted by the native controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandIntent {
    OpenUri(String),
    Copy(String),
    Paste { pane: PaneId, bytes: Vec<u8> },
    Pane(PaneCommand),
    SpawnWindow,
    SetTitle(String),
    Persistence(PersistenceCommand),
}

pub(crate) fn reduce(
    state: &mut WindowState,
    intent: CommandIntent,
    effects: &mut Vec<WindowEffect>,
) {
    match intent {
        CommandIntent::OpenUri(uri) => effects.push(WindowEffect::Uri(UriEffect::Open(uri))),
        CommandIntent::Copy(contents) => {
            effects.push(WindowEffect::Clipboard(ClipboardEffect::Write {
                context: None,
                selection: None,
                contents,
            }));
        }
        CommandIntent::Paste { pane, bytes } => {
            if let Some(pane) = state.panes.get(&pane) {
                effects.push(WindowEffect::Runtime(RuntimePortEffect::SubmitInput {
                    pane: pane.token,
                    bytes,
                }));
            }
        }
        CommandIntent::Pane(command) => panes::reduce_command(state, command, effects),
        CommandIntent::SpawnWindow => effects.push(WindowEffect::Spawn(SpawnEffect::Window)),
        CommandIntent::SetTitle(title) => {
            effects.push(WindowEffect::Window(WindowPortEffect::SetTitle(title)));
        }
        CommandIntent::Persistence(command) => persistence::reduce(state, command, effects),
    }
}
