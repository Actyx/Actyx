use derive_more::Display;

#[derive(Debug, Display, PartialEq, Eq)]
pub enum ParseError {
    #[display(fmt = "Empty string is not permissible for Tag")]
    EmptyTag,
    #[display(fmt = "Empty string is not permissible for AppId")]
    EmptyAppId,
    #[display(fmt = "Invalid AppId: '{}'", _0)]
    InvalidAppId(String),
}

impl std::error::Error for ParseError {}
