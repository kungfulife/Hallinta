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

    pub fn check_workshop_mods_async(&mut self) {
        let steam_path = self.settings.steam_path.clone();
        if steam_path.is_empty() {
            self.invalidate_workshop_check(Some("Steam path not configured".to_string()));
            return;
        }
        let workshop_ids: Vec<String> = self
            .current_mods
            .iter()
            .filter(|m| !m.workshop_id.is_empty() && m.workshop_id != "0")
            .map(|m| m.workshop_id.clone())
            .collect();
        if workshop_ids.is_empty() {
            self.invalidate_workshop_check(None);
            return;
        }
        self.backup_state.workshop_check_generation = self
            .backup_state
            .workshop_check_generation
            .saturating_add(1);
        self.backup_state.workshop_check_in_flight = true;
        self.backup_state.workshop_diagnostic = None;
        let generation = self.backup_state.workshop_check_generation;
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                workshop::check_workshop_mods_installed(&workshop_ids, &steam_path)
            })
            .await
            .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::WorkshopModsChecked { generation, result });
        });
    }

    fn invalidate_workshop_check(&mut self, diagnostic: Option<String>) {
        self.backup_state.workshop_check_generation = self
            .backup_state
            .workshop_check_generation
            .saturating_add(1);
        self.backup_state.workshop_status.clear();
        self.backup_state.workshop_check_in_flight = false;
        self.backup_state.workshop_diagnostic = diagnostic;
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

#[cfg(test)]
mod tests {
    use super::super::test_support::{mod_entry, test_app};

    #[test]
    fn empty_steam_path_invalidates_in_flight_workshop_check() {
        let (_runtime, mut app) = test_app(vec![mod_entry("Alpha", true, "123")]);
        app.backup_state.workshop_check_generation = 7;
        app.backup_state.workshop_check_in_flight = true;
        app.settings.steam_path = String::new();

        app.check_workshop_mods_async();

        assert_eq!(app.backup_state.workshop_check_generation, 8);
        assert!(!app.backup_state.workshop_check_in_flight);
        assert!(app.backup_state.workshop_status.is_empty());
    }

    #[test]
    fn no_workshop_ids_invalidates_in_flight_workshop_check() {
        let (_runtime, mut app) = test_app(vec![mod_entry("Local", true, "0")]);
        app.backup_state.workshop_check_generation = 3;
        app.backup_state.workshop_check_in_flight = true;
        app.settings.steam_path = "C:/Steam".to_string();

        app.check_workshop_mods_async();

        assert_eq!(app.backup_state.workshop_check_generation, 4);
        assert!(!app.backup_state.workshop_check_in_flight);
        assert!(app.backup_state.workshop_status.is_empty());
    }
}
