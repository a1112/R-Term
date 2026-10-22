use crossterm::event::MouseEventKind;

pub(crate) use rterm_runtime::modes::*;

pub(crate) const fn mouse_input_mode_allows(
    mode: MouseInputMode,
    event_kind: MouseEventKind,
) -> bool {
    match mode.reporting() {
        MouseReportingMode::None => false,
        MouseReportingMode::Normal => matches!(
            event_kind,
            MouseEventKind::Down(_)
                | MouseEventKind::Up(_)
                | MouseEventKind::ScrollUp
                | MouseEventKind::ScrollDown
                | MouseEventKind::ScrollLeft
                | MouseEventKind::ScrollRight
        ),
        MouseReportingMode::ButtonEvent => !matches!(event_kind, MouseEventKind::Moved),
        MouseReportingMode::AnyEvent => true,
    }
}
