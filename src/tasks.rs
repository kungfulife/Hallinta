use crate::models::BackupInfo;

/// Results from background tasks dispatched to the tokio runtime.
#[derive(Debug)]
pub enum TaskResult {
    BackupComplete(Result<String, String>),
    RestoreComplete(Result<(), String>),
    SnapshotComplete(Result<String, String>),
    UpgradeBackupComplete(Result<(), String>),
    BackupListLoaded(Result<Vec<BackupInfo>, String>),
    SessionCheckComplete(Result<Vec<crate::models::SessionInfo>, String>),
    SessionListLoaded(Result<Vec<crate::models::SessionInfo>, String>),
    SessionSnapshotsLoaded(Result<Vec<crate::models::SnapshotEntry>, String>),
    WorkshopModsChecked(Result<Vec<(String, bool)>, String>),
    SnapshotCleanupComplete(Result<u32, String>),
    BackupDeleted(Result<String, String>),
    MonitorDataCleared(Result<(), String>),
}
