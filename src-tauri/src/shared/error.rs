//! Usage: Unified application error model (maps internal failures to `CODE: message` strings).

use std::sync::Arc;

pub type AppResult<T> = Result<T, AppError>;

/// Creates an `AppError` with the `DB_ERROR` code.
///
/// Usage: `db_err!("failed to query providers: {}", e)`
macro_rules! db_err {
    ($($arg:tt)*) => {
        $crate::shared::error::AppError::new("DB_ERROR", format!($($arg)*))
    };
}

pub(crate) use db_err;

#[derive(Debug, Clone, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AppError {
    code: String,
    message: String,
    #[source]
    source: Option<Arc<dyn std::error::Error + Send + Sync>>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            source: None,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }
}

fn split_code_message(raw: &str) -> Option<(&str, &str)> {
    let msg = raw.trim();
    let msg = msg.strip_prefix("Error:").unwrap_or(msg).trim();
    if msg.is_empty() {
        return None;
    }

    let (maybe_code, rest) = msg.split_once(':')?;
    let code = maybe_code.trim();
    if code.is_empty() {
        return None;
    }
    let mut chars = code.chars();
    let first = chars.next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    if !chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_') {
        return None;
    }
    Some((code, rest.trim()))
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        if let Some((code, rest)) = split_code_message(&value) {
            return AppError::new(code.to_string(), rest.to_string());
        }
        AppError::new("INTERNAL_ERROR", value)
    }
}

impl From<&'static str> for AppError {
    fn from(value: &'static str) -> Self {
        AppError::from(value.to_string())
    }
}

impl From<AppError> for String {
    fn from(value: AppError) -> Self {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_string_parses_code_and_message() {
        let err = AppError::from("DB_ERROR: failed to open db".to_string());
        assert_eq!(err.to_string(), "DB_ERROR: failed to open db");
    }

    #[test]
    fn from_string_strips_error_prefix() {
        let err = AppError::from("Error: DB_ERROR: failed".to_string());
        assert_eq!(err.to_string(), "DB_ERROR: failed");
    }

    #[test]
    fn from_string_treats_invalid_code_as_internal_error() {
        let err = AppError::from("db_error: failed".to_string());
        assert_eq!(err.to_string(), "INTERNAL_ERROR: db_error: failed");
    }

    #[test]
    fn from_string_trims_message() {
        let err = AppError::from("DB_ERROR:   failed  ".to_string());
        assert_eq!(err.to_string(), "DB_ERROR: failed");
    }

    #[test]
    fn from_string_keeps_code_when_message_is_empty() {
        let err = AppError::from("DB_ERROR:   ".to_string());
        assert_eq!(err.to_string(), "DB_ERROR: ");
    }
}
