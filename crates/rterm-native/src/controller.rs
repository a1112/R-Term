use std::sync::Arc;

use rterm_runtime::{MetadataChange, RuntimeBatch, RuntimeEffectKind, TerminalStateSummary};

use crate::{
    ClipboardEffect, ConfigDiff, HostEffectContext, NotificationEffect, PaneState, PlatformIntent,
    RendererEffect, RuntimePortEffect, TimerIntent, WindowEffect, WindowIntent, WindowPortEffect,
    WindowState, commands, panes,
};

/// Applies one intent atomically and appends typed commands for external owners.
pub fn reduce(state: &mut WindowState, intent: WindowIntent, effects: &mut Vec<WindowEffect>) {
    match intent {
        WindowIntent::Platform(intent) => reduce_platform(state, intent, effects),
        WindowIntent::Command(intent) => commands::reduce(state, intent, effects),
        WindowIntent::RuntimeBatch(batch) => reduce_runtime_batch(state, batch, effects),
        WindowIntent::Config(diff) => reduce_config(state, diff, effects),
        WindowIntent::Timer(intent) => reduce_timer(state, intent, effects),
        WindowIntent::PaneLifecycle(intent) => panes::reduce_lifecycle(state, intent, effects),
        WindowIntent::RedrawRequested => {
            if state.presentation.redraw_pending {
                state.presentation.redraw_pending = false;
                effects.push(WindowEffect::Renderer(RendererEffect::Present));
            }
        }
        WindowIntent::CloseRequested => reduce_close(state, effects),
    }
}

fn reduce_platform(
    state: &mut WindowState,
    intent: PlatformIntent,
    effects: &mut Vec<WindowEffect>,
) {
    match intent {
        PlatformIntent::Focused(focused) => {
            state.platform.focused = focused;
            effects.push(WindowEffect::Window(WindowPortEffect::SetFocused(focused)));
        }
        PlatformIntent::Resized(size) => {
            state.presentation.size = size;
            effects.push(WindowEffect::Renderer(RendererEffect::ResizeSurface(size)));
            request_redraw(state, effects);
        }
    }
}

fn reduce_runtime_batch(
    state: &mut WindowState,
    batch: RuntimeBatch<TerminalStateSummary>,
    effects: &mut Vec<WindowEffect>,
) {
    let pane_id = batch.pane.pane();
    let Some(pane) = state.panes.get(&pane_id) else {
        return;
    };
    if pane.token != batch.pane
        || pane
            .revision
            .is_some_and(|revision| batch.revision <= revision)
    {
        return;
    }
    if batch.snapshot.is_none() && !batch.damage.is_empty() {
        effects.push(WindowEffect::Window(WindowPortEffect::ReportDiagnostic(
            "runtime batch carried damage without a snapshot".to_owned(),
        )));
        return;
    }
    let mut sequence = pane.effect_sequence;
    if let Err(error) = sequence.validate_batch(&batch) {
        effects.push(WindowEffect::Window(WindowPortEffect::ReportDiagnostic(
            error.to_string(),
        )));
        return;
    }

    for runtime_effect in &batch.effects {
        let context = HostEffectContext {
            pane: batch.pane,
            revision: batch.revision,
            sequence: runtime_effect.sequence(),
        };
        effects.push(route_runtime_effect(context, runtime_effect.kind().clone()));
    }

    let metadata_changed = batch.metadata != rterm_runtime::PaneMetadataDelta::default();
    let pane = state
        .panes
        .get_mut(&pane_id)
        .expect("pane generation was checked before mutation");
    pane.effect_sequence = sequence;
    pane.revision = Some(batch.revision);
    pane.restarting = false;
    apply_metadata(pane, batch.metadata);
    if let Some(snapshot) = batch.snapshot {
        let snapshot = Arc::unwrap_or_clone(snapshot);
        pane.snapshot = Some(snapshot);
        effects.push(WindowEffect::Renderer(RendererEffect::ApplyPane {
            pane: batch.pane,
            revision: batch.revision,
            snapshot,
            damage: batch.damage,
        }));
        request_redraw(state, effects);
    } else if metadata_changed {
        request_redraw(state, effects);
    }
}

fn route_runtime_effect(context: HostEffectContext, effect: RuntimeEffectKind) -> WindowEffect {
    match effect {
        RuntimeEffectKind::TransportWrite(bytes) => {
            WindowEffect::Runtime(RuntimePortEffect::WriteTransport { context, bytes })
        }
        RuntimeEffectKind::HostStream(bytes) => {
            WindowEffect::Runtime(RuntimePortEffect::ObserveHostStream { context, bytes })
        }
        RuntimeEffectKind::VisibleOutput(bytes) => {
            WindowEffect::Runtime(RuntimePortEffect::WriteSessionLog { context, bytes })
        }
        RuntimeEffectKind::ModeChange(change) => {
            WindowEffect::Runtime(RuntimePortEffect::ApplyModeChange { context, change })
        }
        RuntimeEffectKind::Bell { count } => {
            WindowEffect::Renderer(RendererEffect::Bell { context, count })
        }
        RuntimeEffectKind::ClipboardWrite {
            selection,
            contents,
        } => WindowEffect::Clipboard(ClipboardEffect::Write {
            context: Some(context),
            selection,
            contents,
        }),
        RuntimeEffectKind::ClipboardRead { selection } => {
            WindowEffect::Clipboard(ClipboardEffect::Read { context, selection })
        }
        RuntimeEffectKind::Notification { title, body } => {
            WindowEffect::Notification(NotificationEffect::Show {
                context,
                title,
                body,
            })
        }
        RuntimeEffectKind::Diagnostic { message } => {
            WindowEffect::Window(WindowPortEffect::RuntimeDiagnostic { context, message })
        }
    }
}

fn apply_metadata(pane: &mut PaneState, metadata: rterm_runtime::PaneMetadataDelta) {
    apply_value(&mut pane.title, metadata.title);
    apply_value(&mut pane.working_directory, metadata.working_directory);
    apply_value(&mut pane.badge_format, metadata.badge_format);
    if let Some(progress) = metadata.progress {
        pane.progress = match progress {
            MetadataChange::Set(progress) => progress,
            MetadataChange::Clear => rterm_runtime::RuntimeProgress::None,
        };
    }
    for change in metadata.user_vars {
        match change.value {
            MetadataChange::Set(value) => {
                pane.user_vars.insert(change.name, value);
            }
            MetadataChange::Clear => {
                pane.user_vars.remove(&change.name);
            }
        }
    }
}

fn apply_value(target: &mut Option<String>, change: Option<MetadataChange<String>>) {
    if let Some(change) = change {
        *target = match change {
            MetadataChange::Set(value) => Some(value),
            MetadataChange::Clear => None,
        };
    }
}

fn reduce_config(state: &mut WindowState, diff: ConfigDiff, effects: &mut Vec<WindowEffect>) {
    if diff.revision <= state.config.revision {
        return;
    }
    let changes = rterm_config::ConfigDiff::between(&state.config.effective, &diff.effective);
    let presentation_changed = changes.font.is_some()
        || changes.window.is_some()
        || changes.render.is_some()
        || state.config.theme != diff.theme;
    state.config.revision = diff.revision;
    state.config.effective = diff.effective;
    state.config.theme.clone_from(&diff.theme);
    if !presentation_changed {
        return;
    }
    effects.push(WindowEffect::Renderer(RendererEffect::ApplyConfig {
        revision: diff.revision,
        theme: diff.theme,
    }));
    request_redraw(state, effects);
}

fn reduce_timer(state: &mut WindowState, intent: TimerIntent, effects: &mut Vec<WindowEffect>) {
    match intent {
        TimerIntent::Arm { timer, epoch } => {
            state.timers.epochs.insert(timer, epoch);
        }
        TimerIntent::Fired { timer, epoch } => {
            if state.timers.epochs.get(&timer) == Some(&epoch) {
                state.timers.epochs.remove(&timer);
                request_redraw(state, effects);
            }
        }
        TimerIntent::Cancel { timer } => {
            state.timers.epochs.remove(&timer);
        }
    }
}

fn reduce_close(state: &mut WindowState, effects: &mut Vec<WindowEffect>) {
    if state.lifecycle.closing {
        return;
    }
    state.lifecycle.closing = true;
    let mut panes = state
        .panes
        .values()
        .map(|pane| pane.token)
        .collect::<Vec<_>>();
    panes.sort_unstable_by_key(|token| token.pane().get());
    for pane in panes {
        effects.push(WindowEffect::Runtime(RuntimePortEffect::BeginClose {
            pane,
        }));
    }
    effects.push(WindowEffect::Window(if state.panes.is_empty() {
        WindowPortEffect::CloseNow
    } else {
        WindowPortEffect::CloseAfterRuntimes
    }));
}

pub(crate) fn request_redraw(state: &mut WindowState, effects: &mut Vec<WindowEffect>) {
    if state.presentation.redraw_pending {
        return;
    }
    state.presentation.redraw_pending = true;
    effects.push(WindowEffect::Window(WindowPortEffect::RequestRedraw));
}
