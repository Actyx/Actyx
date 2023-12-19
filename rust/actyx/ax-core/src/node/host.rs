use super::{node_settings::Settings, node_storage::NodeStorage, settings::system_scope, util::make_keystore};
use crate::{crypto::KeyStoreRef, util::formats::NodeCycleCount};
use anyhow::{Context, Result};
use ax_types::NodeId;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, derive_more::Display)]
#[display(fmt = "data directory `{}` is locked by another AX process", _0)]
pub struct WorkdirLocked(String);
impl std::error::Error for WorkdirLocked {}

pub(crate) struct Host {
    keystore: KeyStoreRef,
    storage: NodeStorage,
    settings_repo: crate::settings::Repository,
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
        let settings_repo = initialize_repository(&base_path)?;
        let storage = initialize_node_storage(&base_path)?;

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

    pub fn get_settings_repo(&self) -> &crate::settings::Repository {
        &self.settings_repo
    }

    pub fn get_settings(&self) -> &crate::node::formats::Settings {
        &self.sys_settings
    }

    pub fn get_cycle_count(&self) -> anyhow::Result<NodeCycleCount> {
        self.storage.get_cycle_count()
    }
}

fn initialize_node_storage(base_path: &Path) -> Result<NodeStorage> {
    Ok(if cfg!(test) {
        NodeStorage::in_memory()
    } else {
        let node_path = base_path.join("node.sqlite");
        NodeStorage::new(node_path.clone()).context(format!("Error opening {}", node_path.display()))?
    })
}

pub fn initialize_repository(base_path: &Path) -> Result<crate::settings::Repository> {
    let settings_db = if cfg!(test) {
        crate::settings::Database::in_memory()?
    } else {
        crate::settings::Database::new(base_path)?
    };
    let mut settings_repo = crate::settings::Repository::new(settings_db);

    // Apply the current schema for com.actyx (it might have changed). If this is
    // unsuccessful, we panic.
    apply_system_schema(&mut settings_repo).expect("Error applying system schema com.actyx.");

    Ok(settings_repo)
}

/// Set the schema for the ActyxOS system settings.
pub(crate) fn apply_system_schema(
    settings_repo: &mut crate::settings::Repository,
) -> Result<(), crate::settings::RepositoryError> {
    tracing::debug!("setting current schema for com.actyx");
    let schema: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/json-schema/node-settings.schema.json"
    )))
    .expect("embedded settings schema is not valid json");
    // check that embedded schema for com.actyx is a valid schema. If not, there is no point in going on.
    crate::settings::Validator::new(schema.clone()).expect("Embedded schema for com.actyx is not a valid JSON schema.");

    settings_repo.set_schema(&system_scope(), schema)?;
    Ok(())
}
