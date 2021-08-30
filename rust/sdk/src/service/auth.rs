use serde::Deserialize;
#[derive(Deserialize, Debug)]
pub struct AuthenticationResponse {
    pub token: String,
}
