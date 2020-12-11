use crate::baltech::BrpResult;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccessoryError {
    #[error("Not a Baltech Reader")]
    NotABaltechReader,
    #[error("No card on the reader")]
    NoCardPresent,
    #[error("Baltech API Error: {0})")]
    BaltechApiError(BrpResult),
    #[error("PCSC Error: {0})")]
    PcscError(pcsc::Error),
}
