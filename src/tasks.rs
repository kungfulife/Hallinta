use crate::models::{BackupInfo, WorkshopCheckReport};

/// Results from background tasks dispatched to the tokio runtime.
#[derive(Debug)]
pub enum TaskResult {
    BackupComplete(Result<String, String>),
    RestoreComplete(Result<(), String>),
    SnapshotComplete(Result<String, String>),
    UpgradeBackupComplete(Result<(), String>),
    BackupListLoaded(Result<Vec<BackupInfo>, String>),
    SessionCheckComplete(Result<Vec<crate::models::SessionInfo>, String>),
    SessionListLoaded {
        result: Result<Vec<crate::models::SessionInfo>, String>,
        /// When false, only update an already-open RestoreManager (live refresh).
        open_if_missing: bool,
    },
    SessionSnapshotsLoaded(Result<Vec<crate::models::SnapshotEntry>, String>),
    WorkshopModsChecked {
        generation: u64,
        result: Result<WorkshopCheckReport, String>,
    },
    SnapshotCleanupComplete(Result<u32, String>),
    BackupDeleted(Result<String, String>),
    MonitorDataCleared(Result<(), String>),
}
