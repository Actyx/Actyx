use actyxos_lib::{ActyxOSError, ActyxOSResult};
pub type Result<T> = ActyxOSResult<T>;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
#[allow(non_camel_case_types)]
pub enum ActyxCliResult<T: Serialize> {
    OK { code: &'static str, result: T },
    ERROR(ActyxOSError),
}
const OK: &str = "OK";
impl<T: Serialize> Into<ActyxCliResult<T>> for ActyxOSResult<T> {
    fn into(self) -> ActyxCliResult<T> {
        match self {
            Ok(result) => ActyxCliResult::OK { code: OK, result },
            Err(err) => ActyxCliResult::ERROR(err),
        }
    }
}
