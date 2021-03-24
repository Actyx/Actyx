use derive_more::Display;

#[derive(Debug, Display)]
pub enum LogsvcdError {
    #[display(fmt = "rusqlite Error occurred: {:?}", _0)]
    RusqliteError(rusqlite::Error),
    #[display(fmt = "serde Error occurred: {:?}", _0)]
    SerdeCborError(serde_cbor::Error),
    #[display(fmt = "serde Error occurred: {:?}", _0)]
    SerdeJsonError(serde_json::Error),
    #[display(fmt = "ioError occurred: {:?}", _0)]
    IoError(std::io::Error),
    #[display(fmt = "Error occurred: {:?}", _0)]
    GenericError(String),
}

impl std::error::Error for LogsvcdError {}

impl From<rusqlite::Error> for LogsvcdError {
    fn from(err: rusqlite::Error) -> LogsvcdError {
        LogsvcdError::RusqliteError(err)
    }
}
impl From<serde_cbor::Error> for LogsvcdError {
    fn from(err: serde_cbor::Error) -> LogsvcdError {
        LogsvcdError::SerdeCborError(err)
    }
}
impl From<serde_json::Error> for LogsvcdError {
    fn from(err: serde_json::Error) -> LogsvcdError {
        LogsvcdError::SerdeJsonError(err)
    }
}
impl From<std::io::Error> for LogsvcdError {
    fn from(err: std::io::Error) -> LogsvcdError {
        LogsvcdError::IoError(err)
    }
}
impl From<String> for LogsvcdError {
    fn from(err: String) -> LogsvcdError {
        LogsvcdError::GenericError(err)
    }
}

impl From<crossbeam::channel::RecvError> for LogsvcdError {
    fn from(err: crossbeam::channel::RecvError) -> LogsvcdError {
        LogsvcdError::GenericError(format!("{}", err))
    }
}
pub type Result<T> = std::result::Result<T, LogsvcdError>;
