use certs::{AppLicenseType, Expiring, SignedAppLicense, SignedAppManifest};
use crypto::PublicKey;

use crate::{
    formats::Licensing,
    rejections::{ApiError, UnauthorizedReason},
};
use chrono::Utc;

pub fn validate_signed_manifest(
    manifest: &SignedAppManifest,
    ax_public_key: &PublicKey,
    licensing: &Licensing,
) -> Result<(), ApiError> {
    manifest
        .validate(ax_public_key)
        .map_err(|x| ApiError::InvalidManifest { msg: x.to_string() })?;
    if licensing.is_node_licensed(ax_public_key)? {
        let app_id = manifest.get_app_id();
        let license = licensing
            .app_id_license(&app_id)
            .ok_or_else(|| ApiError::AppUnauthorized {
                app_id: app_id.clone(),
                reason: UnauthorizedReason::NoLicense,
            })
            .and_then(|license_str| {
                license_str
                    .parse::<SignedAppLicense>()
                    .map_err(|_| ApiError::AppUnauthorized {
                        app_id: app_id.clone(),
                        reason: UnauthorizedReason::MalformedLicense,
                    })
            })?;

        license.validate(ax_public_key).map_err(|_| ApiError::AppUnauthorized {
            app_id,
            reason: UnauthorizedReason::InvalidSignature,
        })?;

        match license.license.license_type {
            AppLicenseType::Expiring(Expiring { expires_at, app_id }) => {
                if app_id != manifest.app_id {
                    Err(ApiError::AppUnauthorized {
                        app_id,
                        reason: UnauthorizedReason::WrongSubject,
                    })
                } else if expires_at < Utc::now() {
                    Err(ApiError::AppUnauthorized {
                        app_id,
                        reason: UnauthorizedReason::Expired,
                    })
                } else {
                    Ok(())
                }
            }
        }
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{formats::Licensing, rejections::ApiError};

    use super::*;
    use actyx_sdk::{app_id, AppId};
    use certs::SignedAppManifest;
    use crypto::{PrivateKey, PublicKey};

    struct TestFixture {
        ax_public_key: PublicKey,
        signed_manifest: SignedAppManifest,
        node_license: String,
        expired_node_license: String,
        app_license: String,
        falsified_app_license: String,
        expired_app_license: String,
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
            node_license: "v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZG5jb20uYWN0eXgubm9kZWlleHBpcmVzQXR0MjA1MC0wMS0wMVQwMDowMDowMFppY3JlYXRlZEF0eB4yMDIyLTAyLTAzVDA3OjE0OjE1LjQ0ODMzMTI4MVppc2lnbmF0dXJleFgvTHgyK1JPVzJaTk1zc2dCK1k4WjFxeVNRbnRFSDRkUm9GRi8zdkVHRFo3Q1pHeXlkdG8zUlBJbStreGd2TkdrM0FMNzM4TSs0UU5oazlvUG5LZjRDZz09aXJlcXVlc3RlcqFlZW1haWxuaW5mb0BhY3R5eC5jb23/".into(),
            expired_node_license: "v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZG5jb20uYWN0eXgubm9kZWlleHBpcmVzQXR0MjAyMC0wMS0wMVQwMDowMDowMFppY3JlYXRlZEF0eB4yMDIyLTAyLTAzVDA3OjE4OjUwLjYwMjYxNDY5MFppc2lnbmF0dXJleFh2Zjh0L3RRQkZxcy9OTDN1TEFjWE5senRlVDFueldZazdBN044a3JpOVBQUmtJb0NZOVVpR0JGNGVPenY0cERSREloZXRUZ1gwM2U5UnZ4MWhiR0hEQT09aXJlcXVlc3RlcqFlZW1haWxuaW5mb0BhY3R5eC5jb23/".into(),
            app_license: "v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZHNjb20uYWN0eXguYXV0aC10ZXN0aWV4cGlyZXNBdHQyMDUwLTAxLTAxVDAwOjAwOjAwWmljcmVhdGVkQXR4HjIwMjItMDItMDNUMDc6MTY6MzkuMjA4MDQ1NjI0WmlzaWduYXR1cmV4WGphWWlENHQxdmF1ZXNldUtMTDhnRU5BZFpPVlNkcXozTmdGVndqYW96M0x2NzcxZnZZQVk3NitoYW5nN2pCaTV1UXhBQmFsMm91azYxTUZXZ2gxMEJnPT1pcmVxdWVzdGVyoWVlbWFpbG5pbmZvQGFjdHl4LmNvbf8=".into(),
            falsified_app_license: "v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZHNjb20uYWN0eXguYXV0aC10ZXN0aWV4cGlyZXNBdHQxOTcxLTAxLTAxVDAwOjAxOjAxWmljcmVhdGVkQXR0MTk3MC0wMS0wMVQwMDowMTowMVppc2lnbmF0dXJleFg1dmEvQ3NYWlk3TUV6VVJ0SUEwVm9mL3R1T3FlejZCN3FYby9JNTl4T0NkUDNwUFVabGZEekZPbExIK09oZXJjWGkwRTJ1RXFnZ2x1cUdyaGFDVVhDZz09aXJlcXVlc3RlcqFlZW1haWx0Y3VzdG9tZXJAZXhhbXBsZS5jb23/".into(),
            expired_app_license: "v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZHNjb20uYWN0eXguYXV0aC10ZXN0aWV4cGlyZXNBdHQyMDIwLTAxLTAxVDAwOjAwOjAwWmljcmVhdGVkQXR4HjIwMjItMDItMDNUMDc6MTc6NDkuNzQ0NDM5NTQ2WmlzaWduYXR1cmV4WFU4S0VjYWxOOTliVlpSOU1nL0hwVCsyT3VjR2dNd2NOa2pkV0Q4cmVBOVJnWmRtTWVjaUlXYysybHlnYTJqMG9tS2RpN3RpRDVvTGV2QXBKcXR6dkJRPT1pcmVxdWVzdGVyoWVlbWFpbG5pbmZvQGFjdHl4LmNvbf8=".into(),
            app_id,
        }
    }

    #[test]
    fn should_succeed_when_node_in_dev_mode() {
        let x = setup();
        validate_signed_manifest(&x.signed_manifest, &x.ax_public_key, &Licensing::default()).unwrap();
    }

    #[test]
    fn should_succeed_when_node_in_prod_mode() {
        let x = setup();
        let mut apps = BTreeMap::new();
        apps.insert(x.app_id, x.app_license);
        validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new(x.node_license, apps),
        )
        .unwrap();
    }

    #[test]
    fn should_fail_when_node_in_prod_mode_without_app_license() {
        let x = setup();
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new(x.node_license, BTreeMap::default()),
        )
        .unwrap_err();
        assert_eq!(
            result,
            ApiError::AppUnauthorized {
                app_id: x.app_id,
                reason: UnauthorizedReason::NoLicense
            }
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
            &Licensing::new(x.node_license, apps),
        )
        .unwrap_err();
        assert_eq!(
            result,
            ApiError::AppUnauthorized {
                app_id: x.app_id,
                reason: UnauthorizedReason::InvalidSignature
            }
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
            &Licensing::new(x.node_license, apps),
        )
        .unwrap_err();
        assert_eq!(
            result,
            ApiError::AppUnauthorized {
                app_id: x.app_id,
                reason: UnauthorizedReason::MalformedLicense
            }
        );
    }

    #[test]
    fn should_fail_when_node_in_prod_mode_with_expired_app_license() {
        let x = setup();
        let mut apps = BTreeMap::new();
        apps.insert(x.app_id.clone(), x.expired_app_license);
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new(x.node_license, apps),
        )
        .unwrap_err();
        assert_eq!(
            result,
            ApiError::AppUnauthorized {
                app_id: x.app_id,
                reason: UnauthorizedReason::Expired
            }
        );
    }

    #[test]
    fn should_fail_when_node_in_prod_mode_with_falsified_node_license() {
        let x = setup();
        let mut apps = BTreeMap::new();
        apps.insert(x.app_id.clone(), x.app_license);
        let mut node_license = base64::decode(&x.node_license).unwrap();
        // change expiration by one year
        for i in 0..node_license.len() - 3 {
            if &node_license[i..i + 4] == b"2050" {
                node_license[i..i + 4].copy_from_slice(b"2049");
            }
        }
        let node_license = base64::encode(node_license);
        assert_ne!(x.node_license, node_license);
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new(node_license, apps),
        )
        .unwrap_err();
        assert_eq!(
            result,
            ApiError::NodeUnauthorized {
                reason: UnauthorizedReason::InvalidSignature
            }
        );
    }

    #[test]
    fn should_fail_when_node_in_prod_mode_with_malformed_node_license() {
        let x = setup();
        let mut apps = BTreeMap::new();
        apps.insert(x.app_id.clone(), x.app_license);
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new("malformed".into(), apps),
        )
        .unwrap_err();
        assert_eq!(
            result,
            ApiError::NodeUnauthorized {
                reason: UnauthorizedReason::MalformedLicense
            }
        );
    }

    #[test]
    fn should_fail_when_node_in_prod_mode_with_expired_node_license() {
        let x = setup();
        let mut apps = BTreeMap::new();
        apps.insert(x.app_id.clone(), x.app_license);
        let result = validate_signed_manifest(
            &x.signed_manifest,
            &x.ax_public_key,
            &Licensing::new(x.expired_node_license, apps),
        )
        .unwrap_err();
        assert_eq!(
            result,
            ApiError::NodeUnauthorized {
                reason: UnauthorizedReason::Expired
            }
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
