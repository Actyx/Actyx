use actyxos_sdk::{app_id, AppManifest};

pub fn app_manifest() -> AppManifest {
    AppManifest::new(
        app_id!("com.example.trial-mode"),
        "display name".into(),
        "0.1.0".into(),
        None,
    )
}
