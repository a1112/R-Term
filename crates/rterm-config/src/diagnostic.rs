#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub path: String,
    pub code: &'static str,
    pub message: String,
}

impl ConfigDiagnostic {
    pub(crate) fn error(
        path: impl Into<String>,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            code,
            message: message.into(),
        }
    }
}
