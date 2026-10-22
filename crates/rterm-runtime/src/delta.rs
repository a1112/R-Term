use rterm_terminal::Terminal;
use rterm_types::DamageRegion;

use crate::{RuntimeProgress, modes::TerminalModeChange};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ByteRange {
    start: usize,
    len: usize,
}

impl ByteRange {
    fn resolve(self, bytes: &[u8]) -> &[u8] {
        &bytes[self.start..self.start + self.len]
    }

    fn resolve_str(self, bytes: &[u8]) -> &str {
        std::str::from_utf8(self.resolve(bytes)).expect("runtime text arena stores valid UTF-8")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectDescriptor {
    ConsoleWrite(ByteRange),
    TransportWrite(ByteRange),
    ModeChange(TerminalModeChange),
    Bell {
        count: u64,
    },
    ClipboardWrite {
        selection: Option<ByteRange>,
        contents: ByteRange,
    },
    ClipboardRead {
        selection: ByteRange,
    },
    Notification {
        title: Option<ByteRange>,
        body: ByteRange,
    },
    Diagnostic {
        message: ByteRange,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetadataDescriptor {
    Set(ByteRange),
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UserVarDescriptor {
    name: ByteRange,
    value: MetadataDescriptor,
}

/// Reserved capacities of every caller-owned runtime arena.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeBufferCapacities {
    byte_arena: usize,
    visible: usize,
    text_arena: usize,
    responses: usize,
    effects: usize,
    damage: usize,
    user_vars: usize,
}

/// Reusable, caller-owned storage populated by [`crate::TerminalRuntime::feed_into`].
#[derive(Debug, Default)]
pub struct RuntimeBuffers {
    bytes: Vec<u8>,
    visible: Vec<u8>,
    text: Vec<u8>,
    responses: Vec<ByteRange>,
    effects: Vec<EffectDescriptor>,
    damage: Vec<DamageRegion>,
    title: Option<MetadataDescriptor>,
    working_directory: Option<MetadataDescriptor>,
    badge_format: Option<MetadataDescriptor>,
    progress: Option<RuntimeProgress>,
    user_vars: Vec<UserVarDescriptor>,
    relocations: u64,
    response_payload_copies: u64,
    owned_response_materializations: u64,
    response_commits: u64,
}

impl RuntimeBuffers {
    /// Preallocates arenas for a steady-state input budget.
    #[must_use]
    pub fn with_capacity(input_bytes: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(input_bytes),
            visible: Vec::with_capacity(input_bytes),
            text: Vec::with_capacity(input_bytes / 8),
            responses: Vec::with_capacity(input_bytes / 64),
            effects: Vec::with_capacity(input_bytes / 64),
            damage: Vec::with_capacity(input_bytes / 64),
            title: None,
            working_directory: None,
            badge_format: None,
            progress: None,
            user_vars: Vec::with_capacity(input_bytes / 128),
            relocations: 0,
            response_payload_copies: 0,
            owned_response_materializations: 0,
            response_commits: 0,
        }
    }

    pub(crate) fn begin_feed(&mut self) -> RuntimeBufferCapacities {
        let capacities = self.capacities();
        self.bytes.clear();
        self.visible.clear();
        self.text.clear();
        self.responses.clear();
        self.effects.clear();
        self.damage.clear();
        self.title = None;
        self.working_directory = None;
        self.badge_format = None;
        self.progress = None;
        self.user_vars.clear();
        self.response_payload_copies = 0;
        self.owned_response_materializations = 0;
        self.response_commits = 0;
        capacities
    }

    pub(crate) fn finish_feed(&mut self, before: RuntimeBufferCapacities) {
        let after = self.capacities();
        self.relocations = self.relocations.saturating_add(u64::from(
            before.byte_arena != after.byte_arena
                || before.visible != after.visible
                || before.text_arena != after.text_arena
                || before.responses != after.responses
                || before.effects != after.effects
                || before.damage != after.damage
                || before.user_vars != after.user_vars,
        ));
    }

    pub(crate) fn visible_mut(&mut self) -> &mut Vec<u8> {
        &mut self.visible
    }

    pub(crate) fn push_transport_write(&mut self, response: &[u8]) {
        self.response_payload_copies = self.response_payload_copies.saturating_add(1);
        self.owned_response_materializations =
            self.owned_response_materializations.saturating_add(1);
        let range = self.push_bytes(response);
        self.commit_transport_write(range);
    }

    pub(crate) fn push_console_write(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let range = self.push_bytes(bytes);
        self.effects.push(EffectDescriptor::ConsoleWrite(range));
    }

    pub(crate) fn push_mode_change(&mut self, change: TerminalModeChange) {
        self.effects.push(EffectDescriptor::ModeChange(change));
    }

    pub(crate) fn try_push_transport_write_with<E>(
        &mut self,
        write: impl FnOnce(&mut Vec<u8>) -> Result<(), E>,
    ) -> Result<(), E> {
        self.responses.reserve(1);
        self.effects.reserve(1);
        let start = self.bytes.len();
        match write(&mut self.bytes) {
            Ok(()) => {
                let range = ByteRange {
                    start,
                    len: self.bytes.len() - start,
                };
                self.commit_transport_write(range);
                Ok(())
            }
            Err(error) => {
                self.bytes.truncate(start);
                Err(error)
            }
        }
    }

    fn commit_transport_write(&mut self, range: ByteRange) {
        self.responses.push(range);
        self.effects.push(EffectDescriptor::TransportWrite(range));
        self.response_commits = self.response_commits.saturating_add(1);
    }

    pub(crate) fn push_bell(&mut self, count: u64) {
        if count != 0 {
            self.effects.push(EffectDescriptor::Bell { count });
        }
    }

    pub(crate) fn push_clipboard_write(&mut self, selection: Option<&str>, contents: &str) {
        let selection = selection.map(|value| self.push_text(value));
        let contents = self.push_text(contents);
        self.effects.push(EffectDescriptor::ClipboardWrite {
            selection,
            contents,
        });
    }

    pub(crate) fn push_clipboard_read(&mut self, selection: &str) {
        let selection = self.push_text(selection);
        self.effects
            .push(EffectDescriptor::ClipboardRead { selection });
    }

    pub(crate) fn push_notification(&mut self, title: Option<&str>, body: &str) {
        let title = title.map(|value| self.push_text(value));
        let body = self.push_text(body);
        self.effects
            .push(EffectDescriptor::Notification { title, body });
    }

    pub(crate) fn push_diagnostic(&mut self, message: &str) {
        let message = self.push_text(message);
        self.effects.push(EffectDescriptor::Diagnostic { message });
    }

    pub(crate) fn damage_mut(&mut self) -> &mut Vec<DamageRegion> {
        &mut self.damage
    }

    pub(crate) fn has_damage(&self) -> bool {
        !self.damage.is_empty()
    }

    pub(crate) fn set_title(&mut self, title: Option<&str>) {
        self.title = Some(match title {
            Some(title) => MetadataDescriptor::Set(self.push_text(title)),
            None => MetadataDescriptor::Clear,
        });
    }

    pub(crate) fn set_working_directory(&mut self, working_directory: Option<&str>) {
        self.working_directory = Some(match working_directory {
            Some(value) => MetadataDescriptor::Set(self.push_text(value)),
            None => MetadataDescriptor::Clear,
        });
    }

    pub(crate) fn set_badge_format(&mut self, badge_format: Option<&str>) {
        self.badge_format = Some(match badge_format {
            Some(value) => MetadataDescriptor::Set(self.push_text(value)),
            None => MetadataDescriptor::Clear,
        });
    }

    pub(crate) const fn set_progress(&mut self, progress: RuntimeProgress) {
        self.progress = Some(progress);
    }

    pub(crate) fn push_user_var(&mut self, name: &str, value: Option<&str>) {
        let name = self.push_text(name);
        let value = match value {
            Some(value) => MetadataDescriptor::Set(self.push_text(value)),
            None => MetadataDescriptor::Clear,
        };
        self.user_vars.push(UserVarDescriptor { name, value });
    }

    fn push_bytes(&mut self, bytes: &[u8]) -> ByteRange {
        let range = ByteRange {
            start: self.bytes.len(),
            len: bytes.len(),
        };
        self.bytes.extend_from_slice(bytes);
        range
    }

    fn push_text(&mut self, text: &str) -> ByteRange {
        let range = ByteRange {
            start: self.text.len(),
            len: text.len(),
        };
        self.text.extend_from_slice(text.as_bytes());
        range
    }

    /// Returns current arena capacities without exposing storage ownership.
    #[must_use]
    pub fn capacities(&self) -> RuntimeBufferCapacities {
        RuntimeBufferCapacities {
            byte_arena: self.bytes.capacity(),
            visible: self.visible.capacity(),
            text_arena: self.text.capacity(),
            responses: self.responses.capacity(),
            effects: self.effects.capacity(),
            damage: self.damage.capacity(),
            user_vars: self.user_vars.capacity(),
        }
    }

    /// Returns the number of feeds that had to grow at least one arena.
    #[must_use]
    pub const fn relocations(&self) -> u64 {
        self.relocations
    }

    /// Payload copies used by compatibility/fallback response producers in the latest feed.
    #[must_use]
    pub const fn response_payload_copies(&self) -> u64 {
        self.response_payload_copies
    }

    #[must_use]
    pub const fn owned_response_materializations(&self) -> u64 {
        self.owned_response_materializations
    }

    #[must_use]
    pub const fn response_commits(&self) -> u64 {
        self.response_commits
    }
}

/// A borrowed metadata value stored in a caller-owned text arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataChangeRef<'a> {
    /// Replace the previous value.
    Set(&'a str),
    /// Remove the previous value.
    Clear,
}

/// Metadata changes produced by one feed.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeMetadataDeltaRef<'a> {
    buffers: &'a RuntimeBuffers,
}

impl<'a> RuntimeMetadataDeltaRef<'a> {
    /// Returns the final title change observed in this feed.
    #[must_use]
    pub fn title(&self) -> Option<MetadataChangeRef<'_>> {
        self.resolve_change(self.buffers.title)
    }

    #[must_use]
    pub fn working_directory(&self) -> Option<MetadataChangeRef<'_>> {
        self.resolve_change(self.buffers.working_directory)
    }

    #[must_use]
    pub fn badge_format(&self) -> Option<MetadataChangeRef<'_>> {
        self.resolve_change(self.buffers.badge_format)
    }

    #[must_use]
    pub const fn progress(&self) -> Option<RuntimeProgress> {
        self.buffers.progress
    }

    pub fn user_vars(self) -> impl Iterator<Item = (&'a str, MetadataChangeRef<'a>)> + 'a {
        self.buffers.user_vars.iter().map(|change| {
            let name = change.name.resolve_str(&self.buffers.text);
            let value = match change.value {
                MetadataDescriptor::Set(range) => {
                    MetadataChangeRef::Set(range.resolve_str(&self.buffers.text))
                }
                MetadataDescriptor::Clear => MetadataChangeRef::Clear,
            };
            (name, value)
        })
    }

    fn resolve_change(&self, change: Option<MetadataDescriptor>) -> Option<MetadataChangeRef<'_>> {
        change.map(|change| match change {
            MetadataDescriptor::Set(range) => {
                MetadataChangeRef::Set(range.resolve_str(&self.buffers.text))
            }
            MetadataDescriptor::Clear => MetadataChangeRef::Clear,
        })
    }

    /// Reports whether this feed changed any published metadata source.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.buffers.title.is_none()
            && self.buffers.working_directory.is_none()
            && self.buffers.badge_format.is_none()
            && self.buffers.progress.is_none()
            && self.buffers.user_vars.is_empty()
    }
}

/// A zero-copy runtime side effect resolved from caller-owned arenas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEffectRef<'a> {
    /// Bytes to write to the terminal-facing console stream, with ANSI controls preserved.
    ConsoleWrite(&'a [u8]),
    TransportWrite(&'a [u8]),
    /// An ordered input-mode change observed while progressing the terminal stream.
    ModeChange(TerminalModeChange),
    Bell {
        count: u64,
    },
    ClipboardWrite {
        selection: Option<&'a str>,
        contents: &'a str,
    },
    ClipboardRead {
        selection: &'a str,
    },
    Notification {
        title: Option<&'a str>,
        body: &'a str,
    },
    Diagnostic {
        message: &'a str,
    },
}

/// Borrowed results of one terminal feed.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeDelta<'a> {
    buffers: &'a RuntimeBuffers,
    bell_count: u64,
    snapshot_changed: bool,
    screen_identity_changed: bool,
}

impl<'a> RuntimeDelta<'a> {
    pub(crate) const fn new(
        buffers: &'a RuntimeBuffers,
        bell_count: u64,
        snapshot_changed: bool,
        screen_identity_changed: bool,
    ) -> Self {
        Self {
            buffers,
            bell_count,
            snapshot_changed,
            screen_identity_changed,
        }
    }

    #[must_use]
    pub fn visible_bytes(self) -> &'a [u8] {
        &self.buffers.visible
    }

    pub fn responses(self) -> impl Iterator<Item = &'a [u8]> + 'a {
        self.buffers
            .responses
            .iter()
            .map(|range| range.resolve(&self.buffers.bytes))
    }

    pub fn console_writes(self) -> impl Iterator<Item = &'a [u8]> + 'a {
        self.effects().filter_map(|effect| match effect {
            RuntimeEffectRef::ConsoleWrite(bytes) => Some(bytes),
            _ => None,
        })
    }

    pub fn mode_changes(self) -> impl Iterator<Item = TerminalModeChange> + 'a {
        self.effects().filter_map(|effect| match effect {
            RuntimeEffectRef::ModeChange(change) => Some(change),
            _ => None,
        })
    }

    pub fn diagnostics(self) -> impl Iterator<Item = &'a str> + 'a {
        self.effects().filter_map(|effect| match effect {
            RuntimeEffectRef::Diagnostic { message } => Some(message),
            _ => None,
        })
    }

    pub fn clipboard_writes(self) -> impl Iterator<Item = (Option<&'a str>, &'a str)> + 'a {
        self.effects().filter_map(|effect| match effect {
            RuntimeEffectRef::ClipboardWrite {
                selection,
                contents,
            } => Some((selection, contents)),
            _ => None,
        })
    }

    pub fn clipboard_reads(self) -> impl Iterator<Item = &'a str> + 'a {
        self.effects().filter_map(|effect| match effect {
            RuntimeEffectRef::ClipboardRead { selection } => Some(selection),
            _ => None,
        })
    }

    pub fn notifications(self) -> impl Iterator<Item = (Option<&'a str>, &'a str)> + 'a {
        self.effects().filter_map(|effect| match effect {
            RuntimeEffectRef::Notification { title, body } => Some((title, body)),
            _ => None,
        })
    }

    pub fn effects(self) -> impl Iterator<Item = RuntimeEffectRef<'a>> + 'a {
        self.buffers.effects.iter().map(|effect| match *effect {
            EffectDescriptor::ConsoleWrite(range) => {
                RuntimeEffectRef::ConsoleWrite(range.resolve(&self.buffers.bytes))
            }
            EffectDescriptor::TransportWrite(range) => {
                RuntimeEffectRef::TransportWrite(range.resolve(&self.buffers.bytes))
            }
            EffectDescriptor::ModeChange(change) => RuntimeEffectRef::ModeChange(change),
            EffectDescriptor::Bell { count } => RuntimeEffectRef::Bell { count },
            EffectDescriptor::ClipboardWrite {
                selection,
                contents,
            } => RuntimeEffectRef::ClipboardWrite {
                selection: selection.map(|range| range.resolve_str(&self.buffers.text)),
                contents: contents.resolve_str(&self.buffers.text),
            },
            EffectDescriptor::ClipboardRead { selection } => RuntimeEffectRef::ClipboardRead {
                selection: selection.resolve_str(&self.buffers.text),
            },
            EffectDescriptor::Notification { title, body } => RuntimeEffectRef::Notification {
                title: title.map(|range| range.resolve_str(&self.buffers.text)),
                body: body.resolve_str(&self.buffers.text),
            },
            EffectDescriptor::Diagnostic { message } => RuntimeEffectRef::Diagnostic {
                message: message.resolve_str(&self.buffers.text),
            },
        })
    }

    #[must_use]
    pub fn damage(self) -> &'a [DamageRegion] {
        &self.buffers.damage
    }

    #[must_use]
    pub const fn bell_count(self) -> u64 {
        self.bell_count
    }

    #[must_use]
    pub const fn metadata(self) -> RuntimeMetadataDeltaRef<'a> {
        RuntimeMetadataDeltaRef {
            buffers: self.buffers,
        }
    }

    #[must_use]
    pub const fn snapshot_changed(self) -> bool {
        self.snapshot_changed
    }

    #[must_use]
    pub const fn screen_identity_changed(self) -> bool {
        self.screen_identity_changed
    }
}

/// Renderer-independent borrowed terminal state.
#[derive(Debug, Clone, Copy)]
pub struct TerminalSnapshotRef<'a> {
    terminal: &'a Terminal,
}

impl<'a> TerminalSnapshotRef<'a> {
    pub(crate) const fn new(terminal: &'a Terminal) -> Self {
        Self { terminal }
    }

    #[must_use]
    pub const fn terminal(self) -> &'a Terminal {
        self.terminal
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeBuffers, RuntimeDelta, RuntimeEffectRef};

    #[test]
    fn failed_response_writer_rolls_back_bytes_and_descriptors_before_next_commit() {
        let mut buffers = RuntimeBuffers::default();
        buffers
            .try_push_transport_write_with::<()>(|bytes| {
                bytes.extend_from_slice(b"first");
                Ok(())
            })
            .unwrap();
        let bytes_len = buffers.bytes.len();
        let responses_len = buffers.responses.len();
        let effects_len = buffers.effects.len();
        let commits = buffers.response_commits();

        let failure = buffers.try_push_transport_write_with(|bytes| {
            bytes.extend_from_slice(b"partial");
            Err("injected writer failure")
        });

        assert_eq!(failure, Err("injected writer failure"));
        assert_eq!(buffers.bytes.len(), bytes_len);
        assert_eq!(buffers.responses.len(), responses_len);
        assert_eq!(buffers.effects.len(), effects_len);
        assert_eq!(buffers.response_commits(), commits);

        buffers
            .try_push_transport_write_with::<()>(|bytes| {
                bytes.extend_from_slice(b"second");
                Ok(())
            })
            .unwrap();
        let delta = RuntimeDelta::new(&buffers, 0, false, false);
        assert_eq!(
            delta.responses().collect::<Vec<_>>(),
            vec![b"first".as_slice(), b"second".as_slice()]
        );
        assert_eq!(
            delta.effects().collect::<Vec<_>>(),
            vec![
                RuntimeEffectRef::TransportWrite(b"first"),
                RuntimeEffectRef::TransportWrite(b"second"),
            ]
        );
        assert_eq!(buffers.response_commits(), commits + 1);
    }
}
