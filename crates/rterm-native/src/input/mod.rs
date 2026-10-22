use std::collections::VecDeque;
use std::io;

use rterm_runtime::SubmitResult;
use rterm_types::TerminalSize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CanonicalizePastedNewlines {
    #[default]
    None,
    LineFeed,
    CarriageReturn,
    CarriageReturnAndLineFeed,
}

impl CanonicalizePastedNewlines {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "None" => Some(Self::None),
            "LineFeed" => Some(Self::LineFeed),
            "CarriageReturn" => Some(Self::CarriageReturn),
            "CarriageReturnAndLineFeed" => Some(Self::CarriageReturnAndLineFeed),
            _ => None,
        }
    }

    #[must_use]
    pub const fn config_text(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::LineFeed => "LineFeed",
            Self::CarriageReturn => "CarriageReturn",
            Self::CarriageReturnAndLineFeed => "CarriageReturnAndLineFeed",
        }
    }
}

#[must_use]
pub fn encode_paste(
    text: &str,
    bracketed_paste: bool,
    canonicalize_newlines: CanonicalizePastedNewlines,
) -> Vec<u8> {
    if !bracketed_paste {
        return canonicalize_pasted_newlines(text, canonicalize_newlines).into_bytes();
    }

    let mut bytes = Vec::with_capacity(b"\x1b[200~".len() + text.len() + b"\x1b[201~".len());
    bytes.extend_from_slice(b"\x1b[200~");
    bytes.extend_from_slice(text.as_bytes());
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}

fn canonicalize_pasted_newlines(
    text: &str,
    canonicalize_newlines: CanonicalizePastedNewlines,
) -> String {
    let replacement = match canonicalize_newlines {
        CanonicalizePastedNewlines::None => return text.to_owned(),
        CanonicalizePastedNewlines::LineFeed => "\n",
        CanonicalizePastedNewlines::CarriageReturn => "\r",
        CanonicalizePastedNewlines::CarriageReturnAndLineFeed => "\r\n",
    };
    let mut normalized = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if matches!(characters.peek(), Some('\n')) {
                    characters.next();
                }
                normalized.push_str(replacement);
            }
            '\n' => normalized.push_str(replacement),
            _ => normalized.push(character),
        }
    }
    normalized
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingPaneCommand {
    Input(Vec<u8>),
    Resize(TerminalSize),
}

impl PendingPaneCommand {
    fn retained_bytes(&self) -> usize {
        match self {
            Self::Input(bytes) => bytes.len(),
            Self::Resize(_) => 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct PendingPaneCommandQueue {
    commands: VecDeque<PendingPaneCommand>,
    retained_bytes: usize,
}

impl PendingPaneCommandQueue {
    pub const MAX_INPUT_CHUNK_BYTES: usize = 64 * 1024;

    #[must_use]
    pub const fn new() -> Self {
        Self {
            commands: VecDeque::new(),
            retained_bytes: 0,
        }
    }

    /// Queues input without spinning when the runtime mailbox is full.
    ///
    /// # Errors
    ///
    /// Returns an error if retained byte accounting overflows or the runtime is closed.
    pub fn submit_input(
        &mut self,
        bytes: &[u8],
        mut submit: impl FnMut(PendingPaneCommand) -> SubmitResult,
    ) -> io::Result<()> {
        let was_empty = self.commands.is_empty();
        for chunk in bytes.chunks(Self::MAX_INPUT_CHUNK_BYTES) {
            self.retained_bytes = self
                .retained_bytes
                .checked_add(chunk.len())
                .ok_or_else(|| io::Error::other("runtime V2 pending input overflow"))?;
            self.commands
                .push_back(PendingPaneCommand::Input(chunk.to_vec()));
        }
        if was_empty {
            self.flush(&mut submit)
        } else {
            Ok(())
        }
    }

    /// Coalesces adjacent resize requests behind any queued input.
    ///
    /// # Errors
    ///
    /// Returns an error if the runtime is closed while flushing the queue.
    pub fn submit_resize(
        &mut self,
        size: TerminalSize,
        mut submit: impl FnMut(PendingPaneCommand) -> SubmitResult,
    ) -> io::Result<()> {
        if let Some(PendingPaneCommand::Resize(pending)) = self.commands.back_mut() {
            *pending = size;
            return Ok(());
        }
        let was_empty = self.commands.is_empty();
        self.commands.push_back(PendingPaneCommand::Resize(size));
        if was_empty {
            self.flush(&mut submit)
        } else {
            Ok(())
        }
    }

    /// Delivers queued commands in FIFO order until accepted, closed, or backpressured.
    ///
    /// # Errors
    ///
    /// Returns `BrokenPipe` when the runtime reports that it is closed.
    pub fn flush(
        &mut self,
        mut submit: impl FnMut(PendingPaneCommand) -> SubmitResult,
    ) -> io::Result<()> {
        while let Some(command) = self.commands.front() {
            match submit(command.clone()) {
                SubmitResult::Accepted => {
                    if let Some(command) = self.commands.pop_front() {
                        self.retained_bytes =
                            self.retained_bytes.saturating_sub(command.retained_bytes());
                    }
                }
                SubmitResult::Backpressured { .. } => break,
                SubmitResult::Closed => {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "runtime V2 pane is closed",
                    ));
                }
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
