use std::env;

static ENV_AZ_STORAGE_SAS_CONNECTION_STRING: &str = "AZ_STORAGE_CONNECTION_STRING";
static ENV_AZ_STORAGE_SAS_TOKEN: &str = "AZ_STORAGE_SAS_TOKEN";

pub struct AzStorageSharedAccessSignature {
    pub connection_string: String,
    pub sas_token: String,
}
pub fn get_az_storage_shared_access_signature() -> Option<AzStorageSharedAccessSignature> {
    match (
        env::var(ENV_AZ_STORAGE_SAS_CONNECTION_STRING),
        env::var(ENV_AZ_STORAGE_SAS_TOKEN),
    ) {
        (Ok(connection_string), Ok(sas_token)) => Some(AzStorageSharedAccessSignature {
            connection_string,
            sas_token,
        }),
        (cs, st) => {
            if cs.is_err() {
                eprintln!("Warning: did not find Azure Storage Shared Access Signature connection string in env; make sure you are logged in");
            }
            if st.is_err() {
                eprintln!("Warning: did not find Azure Storage Shared Access Signature SAS token in env; make sure you are logged in");
            }
            None
        }
    }
}
