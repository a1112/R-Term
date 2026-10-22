use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MIN_COLUMNS: u16 = 2;
pub const MAX_COLUMNS: u16 = 500;
pub const MIN_ROWS: u16 = 1;
pub const MAX_ROWS: u16 = 300;
pub const MAX_INPUT_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_CONTROL_FRAME_BYTES: usize = 8 * 1024;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "open")]
    Open {
        protocol: u16,
        cols: u16,
        rows: u16,
        profile: String,
    },
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
    #[serde(rename = "close")]
    Close,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ServerMessage<'a> {
    #[serde(rename = "opened")]
    Opened {
        protocol: u16,
        #[serde(rename = "sessionId")]
        session_id: &'a str,
        cols: u16,
        rows: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        shell: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<&'a str>,
    },
    #[serde(rename = "exit")]
    Exit { code: u32, signal: Option<&'a str> },
    #[serde(rename = "error")]
    Error {
        code: &'a str,
        message: &'a str,
        fatal: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalDimensions {
    pub cols: u16,
    pub rows: u16,
}

impl TerminalDimensions {
    /// Validates a browser-provided terminal size against the protocol limits.
    ///
    /// # Errors
    ///
    /// Returns which dimension is outside the supported range.
    pub fn validate(cols: u16, rows: u16) -> Result<Self, DimensionError> {
        if !(MIN_COLUMNS..=MAX_COLUMNS).contains(&cols) {
            return Err(DimensionError::Columns(cols));
        }
        if !(MIN_ROWS..=MAX_ROWS).contains(&rows) {
            return Err(DimensionError::Rows(rows));
        }
        Ok(Self { cols, rows })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionError {
    Columns(u16),
    Rows(u16),
}

impl DimensionError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Columns(_) => "INVALID_COLUMNS",
            Self::Rows(_) => "INVALID_ROWS",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientMessage, DimensionError, ServerMessage, TerminalDimensions};

    #[test]
    fn parses_open_message() {
        let message: ClientMessage = serde_json::from_str(
            r#"{"type":"open","protocol":1,"cols":120,"rows":32,"profile":"local-default"}"#,
        )
        .unwrap();
        assert_eq!(
            message,
            ClientMessage::Open {
                protocol: 1,
                cols: 120,
                rows: 32,
                profile: "local-default".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_out_of_range_dimensions() {
        assert_eq!(
            TerminalDimensions::validate(1, 24),
            Err(DimensionError::Columns(1))
        );
        assert_eq!(
            TerminalDimensions::validate(80, 301),
            Err(DimensionError::Rows(301))
        );
    }

    #[test]
    fn serializes_opened_message_with_wire_names() {
        let message = ServerMessage::Opened {
            protocol: 1,
            session_id: "session-1",
            cols: 80,
            rows: 24,
            shell: None,
            cwd: None,
        };
        assert_eq!(
            serde_json::to_string(&message).unwrap(),
            r#"{"type":"opened","protocol":1,"sessionId":"session-1","cols":80,"rows":24}"#
        );
    }

    #[test]
    fn serializes_real_session_metadata_when_available() {
        let message = ServerMessage::Opened {
            protocol: 1,
            session_id: "session-2",
            cols: 120,
            rows: 32,
            shell: Some("zsh"),
            cwd: Some("/project/示例"),
        };
        let value = serde_json::to_value(message).unwrap();
        assert_eq!(value["shell"], "zsh");
        assert_eq!(value["cwd"], "/project/示例");
    }
}
