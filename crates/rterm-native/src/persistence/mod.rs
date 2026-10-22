use crate::{WindowEffect, WindowState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceCommand {
    Save,
}

/// Commands sent to persistence ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceEffect {
    Save,
}

pub(crate) fn reduce(
    _state: &mut WindowState,
    command: PersistenceCommand,
    effects: &mut Vec<WindowEffect>,
) {
    match command {
        PersistenceCommand::Save => {
            effects.push(WindowEffect::Persistence(PersistenceEffect::Save));
        }
    }
}
