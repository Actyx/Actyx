#![allow(clippy::upper_case_acronyms)]
use axossettings::{repository, validation};
use crossbeam::channel::{RecvError, SendError};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

pub type ActyxOSResult<T> = Result<T, ActyxOSError>;
pub fn ax_err<T>(code: ActyxOSCode, message: String) -> ActyxOSResult<T> {
    Err(ActyxOSError { code, message })
}
#[macro_export]
macro_rules! ax_bail {
    ($code:expr, $fmt:expr, $($arg:tt)*) => {
        return ax_err($code, format!($fmt, $($arg)*));
    };
}
pub trait ActyxOSResultExt<T> {
    fn ax_err(self, code: ActyxOSCode) -> ActyxOSResult<T>;
    fn ax_internal(self) -> ActyxOSResult<T>;
    fn ax_invalid_input(self) -> ActyxOSResult<T>;
    fn ax_err_ctx(self, code: ActyxOSCode, ctx: impl Into<String>) -> ActyxOSResult<T>;
}

impl<T, E: Display> ActyxOSResultExt<T> for Result<T, E> {
    fn ax_err(self, code: ActyxOSCode) -> ActyxOSResult<T> {
        self.map_err(|e| ActyxOSError {
            code,
            message: e.to_string(),
        })
    }
    fn ax_internal(self) -> ActyxOSResult<T> {
        self.map_err(|e| ActyxOSError {
            code: ERR_INTERNAL_ERROR,
            message: e.to_string(),
        })
    }
    fn ax_invalid_input(self) -> ActyxOSResult<T> {
        self.map_err(|e| ActyxOSError {
            code: ERR_INVALID_INPUT,
            message: e.to_string(),
        })
    }
    fn ax_err_ctx(self, code: ActyxOSCode, ctx: impl Into<String>) -> ActyxOSResult<T> {
        self.map_err(move |e| ActyxOSError::new(code, format!("{} ({})", ctx.into(), e)))
    }
}
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum ActyxOSCode {
    ERR_FILE_EXISTS,
    ERR_IO,
    ERR_UNAUTHORIZED,
    ERR_USER_UNAUTHENTICATED,
    ERR_INTERNAL_ERROR,
    ERR_NODE_UNREACHABLE,
    ERR_PATH_INVALID,
    ERR_SETTINGS_INVALID,
    ERR_SETTINGS_INVALID_SCHEMA,
    ERR_SETTINGS_UNKNOWN_SCOPE,
    ERR_SETTINGS_INVALID_AT_SCOPE,
    ERR_SETTINGS_NOT_FOUND_AT_SCOPE,
    ERR_INVALID_INPUT,
    // Fatal Error, the state on disk is inconsistent
    ERR_INVALID_NODE_STATE,
}
impl ActyxOSCode {
    pub fn with_message(self, message: impl Into<String>) -> ActyxOSError {
        ActyxOSError {
            code: self,
            message: message.into(),
        }
    }
}
use ActyxOSCode::*;
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ActyxOSError {
    code: ActyxOSCode,
    message: String,
}
impl std::error::Error for ActyxOSError {}
impl ActyxOSError {
    pub fn new(code: ActyxOSCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    /// Shortcut to create an internal error, since this is very common
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ActyxOSCode::ERR_INTERNAL_ERROR, message)
    }
    pub fn code(&self) -> ActyxOSCode {
        self.code
    }
}

impl From<RecvError> for ActyxOSError {
    fn from(err: RecvError) -> ActyxOSError {
        ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("{}", err))
    }
}
impl<T> From<SendError<T>> for ActyxOSError {
    fn from(err: SendError<T>) -> ActyxOSError {
        ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("{}", err))
    }
}

impl From<std::io::Error> for ActyxOSError {
    fn from(err: std::io::Error) -> ActyxOSError {
        // TODO: improve error message
        ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("{}", err))
    }
}
impl From<repository::Error> for ActyxOSError {
    fn from(err: repository::Error) -> ActyxOSError {
        let code = match err {
            repository::Error::SchemaNotFound(_) => ActyxOSCode::ERR_SETTINGS_UNKNOWN_SCOPE,
            repository::Error::NoValidSettings(_) => ActyxOSCode::ERR_SETTINGS_INVALID_AT_SCOPE,
            repository::Error::NoSettingsAtScope(_) => ActyxOSCode::ERR_SETTINGS_NOT_FOUND_AT_SCOPE,
            repository::Error::ValidationError(ref validation_err) => match validation_err {
                validation::Error::InvalidSchema(_) => ActyxOSCode::ERR_SETTINGS_INVALID_SCHEMA,
                validation::Error::ValidationFailed(_) => ActyxOSCode::ERR_SETTINGS_INVALID,
                validation::Error::MissingDefault(_) => ActyxOSCode::ERR_SETTINGS_INVALID,
            },
            _ => ActyxOSCode::ERR_INTERNAL_ERROR,
        };
        code.with_message(format!("{}", err))
    }
}
impl Display for ActyxOSError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.code {
           ERR_SETTINGS_INVALID => write!(f, "[ERR_SETTINGS_INVALID] Error: {}", self.message),
           ERR_SETTINGS_INVALID_SCHEMA => write!(f, "[ERR_SETTINGS_INVALID_SCHEMA] Error: {}", self.message),
           ERR_SETTINGS_UNKNOWN_SCOPE => write!(f, "[ERR_SETTINGS_UNKNOWN_SCOPE] Error: {}", self.message),
           ERR_SETTINGS_INVALID_AT_SCOPE => write!(f, "[ERR_SETTINGS_INVALID_AT_SCOPE] Error: {}", self.message),
           ERR_SETTINGS_NOT_FOUND_AT_SCOPE => write!(f, "[ERR_SETTINGS_NOT_FOUND_AT_SCOPE] Error: {}", self.message),
           ERR_INVALID_INPUT => write!(f, "[ERR_INVALID_INPUT] Error: {}", self.message),
           ERR_INVALID_NODE_STATE => write!(f, "[ERR_INVALID_NODE_STATE] Error the state of your Node is inconsitent. Please file a bug at http://developer.actyx.com. message: {}", self.message),
           ERR_INTERNAL_ERROR => write!(f, "[ERR_INTERNAL_ERROR] Error: internal error. Please file a bug at http://developer.actyx.com. message: {}", self.message),
           ERR_NODE_UNREACHABLE => write!(f, "[ERR_NODE_UNREACHABLE] Error: unable to reach node, additional message: {}", self.message),
           ERR_UNAUTHORIZED => write!(f, "[ERR_UNAUTHORIZED] Error: {}", self.message),
           ERR_USER_UNAUTHENTICATED => write!(f, "[ERR_USER_UNAUTHENTICATED] Error: {}", self.message),
           ERR_FILE_EXISTS => write!(f, "[ERR_FILE_EXISTS] Error: {}", self.message),
           ERR_PATH_INVALID => write!(f, "[ERR_PATH_INVALID] Error: {}", self.message),
           ERR_IO => write!(f, "[ERR_IO]: Error: {}", self.message),
       }
    }
}
