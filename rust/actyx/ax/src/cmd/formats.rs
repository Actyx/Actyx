use util::formats::{ActyxOSError, ActyxOSResult};
pub type Result<T> = ActyxOSResult<T>;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum ActyxCliResult<T: Serialize> {
    OK { code: &'static str, result: T },
    ERROR(ActyxOSError),
}
const OK: &str = "OK";
impl<T: Serialize> From<ActyxOSResult<T>> for ActyxCliResult<T> {
    fn from(res: ActyxOSResult<T>) -> Self {
        match res {
            Ok(result) => ActyxCliResult::OK { code: OK, result },
            Err(err) => ActyxCliResult::ERROR(err),
        }
    }
}
