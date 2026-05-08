use super::HallintaApp;
use crate::core::{backup, save_monitor, workshop};
use crate::tasks::TaskResult;

impl HallintaApp {
    // ── Async Task Dispatchers ────────────────────────────────────────

    pub fn load_backup_list_async(&self) {
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(backup::list_backups)
                .await
                .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::BackupListLoaded(result));
        });
    }

    pub fn load_sessions_async(&self) {
        let preset = self.selected_preset.clone();
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || save_monitor::list_sessions(&preset))
                .await
                .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::SessionListLoaded(result));
        });
    }

    pub fn load_session_snapshots_async(&self, preset: String, session_id: String) {
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                save_monitor::list_session_snapshots(&preset, &session_id)
            })
            .await
            .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::SessionSnapshotsLoaded(result));
        });
    }

    pub fn check_workshop_mods_async(&self) {
        let steam_path = self.settings.steam_path.clone();
        if steam_path.is_empty() {
            return;
        }
        let workshop_ids: Vec<String> = self
            .current_mods
            .iter()
            .filter(|m| !m.workshop_id.is_empty() && m.workshop_id != "0")
            .map(|m| m.workshop_id.clone())
            .collect();
        if workshop_ids.is_empty() {
            return;
        }
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                workshop::check_workshop_mods_installed(&workshop_ids, &steam_path)
            })
            .await
            .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::WorkshopModsChecked(result));
        });
    }

    pub fn delete_backup_async(&self, filename: String) {
        let tx = self.task_tx.clone();
        let fname = filename.clone();
        self.async_runtime.spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || backup::delete_backup(&fname).map(|()| fname))
                    .await
                    .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::BackupDeleted(result));
        });
    }

    pub fn clear_monitor_data_async(&self) {
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(save_monitor::clear_monitor_data)
                .await
                .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::MonitorDataCleared(result));
        });
    }
}
