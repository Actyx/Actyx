use crate::{node_settings::Settings, node_storage::NodeStorage, settings::system_scope, util::make_keystore};
use actyx_sdk::NodeId;
use anyhow::{Context, Result};
use crypto::KeyStoreRef;
use derive_more::Display;
use std::path::PathBuf;
use util::formats::NodeCycleCount;

#[derive(Debug, Clone, Display)]
#[display(fmt = "data directory `{}` is locked by another Actyx process", _0)]
pub struct WorkdirLocked(String);
impl std::error::Error for WorkdirLocked {}

pub(crate) struct Host {
    keystore: KeyStoreRef,
    storage: NodeStorage,
    settings_repo: settings::Repository,
    sys_settings: Settings,
}
#[cfg(not(target_os = "android"))]
pub fn lock_working_dir(working_dir: impl AsRef<std::path::Path>) -> anyhow::Result<fslock::LockFile> {
    let mut lf =
        fslock::LockFile::open(&working_dir.as_ref().join("lockfile")).context("Error opening file `lockfile`")?;
    if !lf.try_lock().context("Error locking file `lockfile`")? {
        return Err(WorkdirLocked(working_dir.as_ref().display().to_string()).into());
    }
    Ok(lf)
}
impl Host {
    pub fn new(base_path: PathBuf) -> Result<Self> {
        let (settings_db, storage) = if cfg!(test) {
            (settings::Database::in_memory()?, NodeStorage::in_memory())
        } else {
            (
                settings::Database::new(&base_path)?,
                NodeStorage::new(base_path.join("node.sqlite")).context("Error opening node.sqlite")?,
            )
        };
        let mut settings_repo = settings::Repository::new(settings_db);
        // Apply the current schema for com.actyx (it might have changed). If this is
        // unsuccessful, we panic.
        apply_system_schema(&mut settings_repo).expect("Error applying system schema com.actyx.");

        let sys_settings_json = settings_repo
            .get_settings(&system_scope(), false)
            .context("Unable to get initial system settings")?;
        let sys_settings: Settings =
            serde_json::from_value(sys_settings_json).context("Deserializing system settings json")?;

        let keystore = make_keystore(storage.clone())?;

        Ok(Self {
            keystore,
            storage,
            settings_repo,
            sys_settings,
        })
    }

    /// Returns this node's NodeId
    pub fn get_or_create_node_id(&self) -> anyhow::Result<NodeId> {
        if let Some(key_id) = self.storage.get_node_key()? {
            Ok(key_id)
        } else {
            let node_id: NodeId = self.keystore.write().generate_key_pair()?.into();
            self.storage.set_node_id(node_id)?;
            Ok(node_id)
        }
    }

    pub fn get_keystore(&self) -> KeyStoreRef {
        self.keystore.clone()
    }

    pub fn get_settings_repo(&self) -> &settings::Repository {
        &self.settings_repo
    }

    pub fn get_settings(&self) -> &Settings {
        &self.sys_settings
    }

    pub fn get_cycle_count(&self) -> anyhow::Result<NodeCycleCount> {
        self.storage.get_cycle_count()
    }
}

/// Set the schema for the ActyxOS system settings.
pub(crate) fn apply_system_schema(settings_repo: &mut settings::Repository) -> Result<(), settings::RepositoryError> {
    tracing::debug!("setting current schema for com.actyx");
    let schema: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../protocols/json-schema/node-settings.schema.json"
    )))
    .expect("embedded settings schema is not valid json");
    // check that embedded schema for com.actyx is a valid schema. If not, there is no point in going on.
    settings::Validator::new(schema.clone()).expect("Embedded schema for com.actyx is not a valid JSON schema.");

    settings_repo.set_schema(&system_scope(), schema)?;
    Ok(())
}
