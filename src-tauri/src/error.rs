use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<&'static str>,
}

impl CommandError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: false,
            recovery: None,
        }
    }

    pub fn retryable(mut self, recovery: Option<&'static str>) -> Self {
        self.retryable = true;
        self.recovery = recovery;
        self
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for CommandError {}

impl From<tauri::Error> for CommandError {
    fn from(_: tauri::Error) -> Self {
        Self::new("NATIVE_OPERATION_FAILED", "The native operation failed")
    }
}
