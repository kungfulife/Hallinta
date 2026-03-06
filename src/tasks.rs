use crate::models::{BackupInfo, Catalog, MonitorSnapshot};

/// Results from background tasks dispatched to the tokio runtime.
#[derive(Debug)]
pub enum TaskResult {
    BackupComplete(Result<String, String>),
    RestoreComplete(Result<(), String>),
    SnapshotComplete(Result<String, String>),
    CatalogFetched(Result<Catalog, String>),
    PresetDownloaded(Result<String, String>),
    UpgradeBackupComplete(Result<(), String>),
    AutoBackupComplete(Result<String, String>),
    BackupListLoaded(Result<Vec<BackupInfo>, String>),
    SnapshotListLoaded(Result<Vec<MonitorSnapshot>, String>),
    WorkshopModsChecked(Result<Vec<(String, bool)>, String>),
    BackupCleanupComplete(Result<u32, String>),
    SnapshotCleanupComplete(Result<u32, String>),
    BackupDeleted(Result<String, String>),
    MonitorDataCleared(Result<(), String>),
}
