use certs::{SignedAppLicense, SignedAppManifest};
use crypto::PublicKey;

use crate::{formats::Licensing, rejections::ApiError};

pub fn validate_signed_manifest(
    manifest: &SignedAppManifest,
    ax_public_key: &PublicKey,
    licensing: &Licensing,
) -> Result<(), ApiError> {
    manifest
        .validate(ax_public_key)
        .map_err(|x| ApiError::InvalidManifest { msg: x.to_string() })?;
    if licensing.is_node_licensed() {
        let app_id = manifest.get_app_id();
        let license = licensing
            .app_id_license(&app_id)
            .ok_or_else(|| ApiError::AppUnauthorized {
                app_id: app_id.clone(),
                reason: "License not found for app".into(),
            })
            .and_then(|license_str| {
                license_str
                    .parse::<SignedAppLicense>()
                    .map_err(|_| ApiError::AppUnauthorized {
                        app_id: app_id.clone(),
                        reason: "Could not parse license".into(),
                    })
            })?;

        license.validate(ax_public_key).map_err(|_| ApiError::AppUnauthorized {
            app_id,
            reason: "Could not validate license".into(),
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{formats::Licensing, rejections::ApiError};

    use super::validate_signed_manifest;
    use actyx_sdk::{app_id, AppId};
    use certs::SignedAppManifest;
    use crypto::{PrivateKey, PublicKey};

    struct TestFixture {
        ax_public_key: PublicKey,
        signed_manifest: SignedAppManifest,
        app_license: String,
        falsified_app_license: String,
        app_id: AppId,
    }

    fn setup() -> TestFixture {
        let ax_private_key: PrivateKey = "0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w=".parse().unwrap();
        let app_id = app_id!("com.actyx.auth-test");
        let serialized_manifest = serde_json::json!({
            "appId": app_id,
            "displayName": "auth test app",
            "version": "v0.0.1",
            "signature": "v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w=="
        });
        TestFixture {
            ax_public_key: ax_private_key.into(),
            signed_manifest: serde_json::from_value(serialized_manifest).unwrap(),
            app_license: "v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZHNjb20uYWN0eXguYXV0aC10ZXN0aWV4cGlyZXNBdHQxOTcxLTAxLTAxVDAwOjAxOjAxWmljcmVhdGVkQXR0MTk3MC0wMS0wMVQwMDowMTowMVppc2lnbmF0dXJleFhBQWRSd1U4UTZlb3JLY0N3SjE1T0t4OWVPQ0kxNjN3MFhwTFpHWkNPUWlDWUZlYkR1cFlBbWlNOVhsb3dDYWw5dUtuSWhRelkzSUo2RkdUbEtJMStEUT09aXJlcXVlc3RlcqFlZW1haWx0Y3VzdG9tZXJAZXhhbXBsZS5jb23/".into(),
            falsified_app_license: "v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZHNjb20uYWN0eXguYXV0aC10ZXN0aWV4cGlyZXNBdHQxOTcxLTAxLTAxVDAwOjAxOjAxWmljcmVhdGVkQXR0MTk3MC0wMS0wMVQwMDowMTowMVppc2lnbmF0dXJleFg1dmEvQ3NYWlk3TUV6VVJ0SUEwVm9mL3R1T3FlejZCN3FYby9JNTl4T0NkUDNwUFVabGZEekZPbExIK09oZXJjWGkwRTJ1RXFnZ2x1cUdyaGFDVVhDZz09aXJlcXVlc3RlcqFlZW1haWx0Y3VzdG9tZXJAZXhhbXBsZS5jb23/".into(),
            app_id,
        }
    }

    #[test]
    fn should_succeed_when_node_in_dev_mode() {
        let x = setup();
        let result = validate_signed_manifest(&x.signed_manifest, &x.ax_public_key, &Licensing::default());
        assert!(result.is_ok());
    }

    #[test]
    fn should_succeed_when_node_in_prod_mode() {
        let x = setup();
        let mut apps = BTreeMap::new();
        apps.insert(x.app_id, x.app_license);
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new("prod mode".into(), apps),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn should_fail_when_node_in_prod_mode_without_app_license() {
        let x = setup();
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new("prod mode".into(), BTreeMap::default()),
        )
        .unwrap_err();
        assert!(
            matches!(result, ApiError::AppUnauthorized { app_id, reason } if reason == "License not found for app" && app_id == x.app_id)
        );
    }

    #[test]
    fn should_fail_when_node_in_prod_mode_with_falsified_app_license() {
        let x = setup();
        let mut apps = BTreeMap::new();
        apps.insert(x.app_id.clone(), x.falsified_app_license);
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new("prod mode".into(), apps),
        )
        .unwrap_err();
        println!("{:?}", result);
        assert!(
            matches!(result, ApiError::AppUnauthorized { app_id, reason } if reason == "Could not validate license" && app_id == x.app_id)
        );
    }

    #[test]
    fn should_fail_when_node_in_prod_mode_with_malformed_app_license() {
        let x = setup();
        let mut apps = BTreeMap::new();
        apps.insert(x.app_id.clone(), "malformed license".to_owned());
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new("prod mode".into(), apps),
        )
        .unwrap_err();
        assert!(
            matches!(result, ApiError::AppUnauthorized { app_id, reason } if reason == "Could not parse license" && app_id == x.app_id)
        );
    }

    #[test]
    fn should_fail_when_ax_public_key_is_wrong() {
        let x = setup();
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &PrivateKey::generate().into(),
            &Licensing::default(),
        )
        .unwrap_err();
        assert!(
            matches!(result, ApiError::InvalidManifest { msg} if msg == "Failed to validate developer certificate. Invalid signature for provided input.")
        );
    }
}
