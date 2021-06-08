mod app_domain;
mod developer_certificate;
mod signature;
mod signed_app_manifest;
mod trial_app_manifest;

pub use app_domain::AppDomain;
pub use developer_certificate::{DeveloperCertificate, ManifestDeveloperCertificate};
pub use signed_app_manifest::SignedAppManifest;
pub use trial_app_manifest::TrialAppManifest;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AppManifest {
    // NB! Signed needs to come before Trial, due to how serde deserialize untagged enums
    Signed(SignedAppManifest),
    Trial(TrialAppManifest),
}
