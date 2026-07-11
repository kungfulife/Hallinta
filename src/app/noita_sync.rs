use super::HallintaApp;
use super::import_export::with_file_rollback;
use crate::core::{logging, mods, platform, presets, settings};
use crate::models::{ModEntry, Modal, NoitaSyncState};
use std::path::{Path, PathBuf};

impl HallintaApp {
    pub(crate) fn is_noita_sync_live(&self) -> bool {
        self.noita_sync_state == NoitaSyncState::Live
    }

    pub(crate) fn enter_configuration_only(&mut self, message: String) {
        if self.noita_sync_state == NoitaSyncState::ConfigurationOnly
            && self.settings.needs_noita_reconciliation
        {
            self.noita_directory_error = Some(message);
            return;
        }

        self.file_watcher.pending_external_mods = None;
        if self.save_monitor.is_running() {
            self.stop_save_monitor();
        }
        self.noita_sync_state = NoitaSyncState::ConfigurationOnly;
        self.settings.needs_noita_reconciliation = true;
        self.noita_directory_error = Some(message);
        let _ = settings::save_settings(&self.settings);
    }

    pub(crate) fn show_noita_reconciliation(&mut self, file_mods: Vec<ModEntry>) {
        let already_pending = self.noita_sync_state == NoitaSyncState::ReconciliationPending
            && self.settings.needs_noita_reconciliation;
        self.noita_sync_state = NoitaSyncState::ReconciliationPending;
        self.settings.needs_noita_reconciliation = true;
        self.noita_directory_error = None;
        if !already_pending {
            let _ = settings::save_settings(&self.settings);
        }
        let summary =
            super::timers::build_external_mod_change_summary(&self.current_mods, &file_mods);
        self.active_modal = Some(Modal::NoitaReconciliation {
            file_mods,
            summary,
            error: None,
        });
    }

    pub(crate) fn apply_selected_preset_to_noita(&mut self) -> Result<(), String> {
        let noita_dir = self.settings.noita_dir.clone();
        if !platform::is_configured_path(&noita_dir) {
            let error = super::noita_directory_error_message(&noita_dir, "");
            self.enter_configuration_only(error.clone());
            return Err(error);
        }

        let dir = PathBuf::from(&noita_dir);
        if let Err(error) = mods::load_mod_config(&dir) {
            let message = super::noita_directory_error_message(&noita_dir, &error);
            self.enter_configuration_only(message.clone());
            return Err(message);
        }
        let xml = mods::mods_to_xml(&self.current_mods);
        let mut new_presets = self.presets.clone();
        new_presets.insert(self.selected_preset.clone(), self.current_mods.clone());
        let mut new_settings = self.settings.clone();
        new_settings.selected_preset = self.selected_preset.clone();
        new_settings.needs_noita_reconciliation = false;
        let data_dir = settings::get_data_dir()?;
        let paths = [
            dir.join("mod_config.xml"),
            data_dir.join("presets.json"),
            data_dir.join("settings.json"),
        ];
        with_file_rollback(&paths, || {
            mods::write_mod_config(&dir, &xml)?;
            presets::save_presets(&new_presets)?;
            settings::save_settings(&new_settings)
        })?;

        self.presets = new_presets;
        self.settings = new_settings;
        self.finish_noita_reconciliation();
        self.refresh_noita_config_mtime(&dir);
        let _ = logging::log(
            "INFO",
            &format!("Applied preset \"{}\" to Noita", self.selected_preset),
            "ModManager",
        );
        Ok(())
    }

    pub(crate) fn save_detected_noita_as_preset(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a preset name.".to_string());
        }
        if self.presets.contains_key(name) {
            return Err(format!("A preset named \"{name}\" already exists."));
        }

        let noita_dir = self.settings.noita_dir.clone();
        let dir = PathBuf::from(&noita_dir);
        let latest_mods = match mods::load_mod_config(&dir) {
            Ok(mods) => mods,
            Err(error) => {
                let message = super::noita_directory_error_message(&noita_dir, &error);
                self.enter_configuration_only(message.clone());
                return Err(message);
            }
        };
        let mut new_presets = self.presets.clone();
        new_presets.insert(name.to_string(), latest_mods.clone());
        let mut new_settings = self.settings.clone();
        new_settings.selected_preset = name.to_string();
        new_settings.needs_noita_reconciliation = false;
        let data_dir = settings::get_data_dir()?;
        let paths = [
            data_dir.join("presets.json"),
            data_dir.join("settings.json"),
        ];
        with_file_rollback(&paths, || {
            presets::save_presets(&new_presets)?;
            settings::save_settings(&new_settings)
        })?;

        self.current_mods = latest_mods;
        self.selected_preset = name.to_string();
        self.presets = new_presets;
        self.settings = new_settings;
        self.finish_noita_reconciliation();
        self.refresh_noita_config_mtime(&dir);
        self.check_workshop_mods_async();
        let _ = logging::log(
            "INFO",
            &format!("Saved detected Noita setup as preset \"{name}\""),
            "PresetManager",
        );
        Ok(())
    }

    pub(crate) fn default_detected_preset_name(&self) -> String {
        let base = "Detected Noita Setup";
        if !self.presets.contains_key(base) {
            return base.to_string();
        }
        (2..)
            .map(|suffix| format!("{base} ({suffix})"))
            .find(|name| !self.presets.contains_key(name))
            .expect("an unused preset name should always exist")
    }

    fn finish_noita_reconciliation(&mut self) {
        self.noita_sync_state = NoitaSyncState::Live;
        self.settings.needs_noita_reconciliation = false;
        self.settings.selected_preset = self.selected_preset.clone();
        self.noita_directory_error = None;
        self.file_watcher.pending_external_mods = None;
    }

    fn refresh_noita_config_mtime(&mut self, dir: &Path) {
        if let Ok(mtime) = mods::get_file_modified_time(&dir.join("mod_config.xml")) {
            self.file_watcher.last_modified_time = mtime;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{mod_entry, test_app};
    use super::*;

    fn temp_config_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("hallinta-{label}-{}", std::process::id()))
    }

    #[test]
    fn repeated_configuration_only_entry_updates_only_the_error() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.noita_sync_state = NoitaSyncState::ConfigurationOnly;
        app.settings.needs_noita_reconciliation = true;
        app.file_watcher.pending_external_mods = Some(vec![mod_entry("Pending", true, "1")]);

        app.enter_configuration_only("new failure detail".to_string());

        assert_eq!(
            app.noita_directory_error.as_deref(),
            Some("new failure detail")
        );
        assert!(
            app.file_watcher.pending_external_mods.is_some(),
            "an already-active configuration-only state should not run transition cleanup again"
        );
    }

    #[test]
    fn reshown_reconciliation_does_not_resave_settings() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.noita_sync_state = NoitaSyncState::ReconciliationPending;
        app.settings.needs_noita_reconciliation = true;
        settings::reset_save_settings_call_count();

        app.show_noita_reconciliation(vec![mod_entry("Disk", true, "1")]);

        assert_eq!(settings::save_settings_call_count(), 0);
        assert!(matches!(
            app.active_modal,
            Some(Modal::NoitaReconciliation { .. })
        ));
    }

    #[test]
    fn configuration_only_save_does_not_touch_mod_config() {
        let dir = temp_config_dir("configuration-only-save");
        std::fs::create_dir_all(&dir).unwrap();
        mods::write_mod_config(&dir, &mods::mods_to_xml(&[mod_entry("Disk", true, "1")])).unwrap();
        let (_runtime, mut app) = test_app(vec![mod_entry("Local", false, "2")]);
        app.settings.noita_dir = dir.to_string_lossy().to_string();
        app.settings.needs_noita_reconciliation = true;
        app.noita_sync_state = NoitaSyncState::ConfigurationOnly;

        app.try_save_mod_config_and_preset().unwrap();

        assert_eq!(mods::load_mod_config(&dir).unwrap()[0].name, "Disk");
        assert_eq!(app.presets["Default"][0].name, "Local");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn applying_selected_preset_finishes_reconciliation() {
        let dir = temp_config_dir("apply-selected-reconciliation");
        std::fs::create_dir_all(&dir).unwrap();
        mods::write_mod_config(&dir, &mods::mods_to_xml(&[mod_entry("Disk", true, "1")])).unwrap();
        let (_runtime, mut app) = test_app(vec![mod_entry("Selected", false, "2")]);
        app.settings.noita_dir = dir.to_string_lossy().to_string();
        app.settings.needs_noita_reconciliation = true;
        app.noita_sync_state = NoitaSyncState::ReconciliationPending;

        app.apply_selected_preset_to_noita().unwrap();

        assert_eq!(mods::load_mod_config(&dir).unwrap()[0].name, "Selected");
        assert_eq!(app.noita_sync_state, NoitaSyncState::Live);
        assert!(!app.settings.needs_noita_reconciliation);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detected_setup_becomes_new_preset_without_changing_noita() {
        let dir = temp_config_dir("capture-detected-reconciliation");
        std::fs::create_dir_all(&dir).unwrap();
        let detected = vec![mod_entry("Detected", true, "1")];
        mods::write_mod_config(&dir, &mods::mods_to_xml(&detected)).unwrap();
        let (_runtime, mut app) = test_app(vec![mod_entry("Local", false, "2")]);
        app.settings.noita_dir = dir.to_string_lossy().to_string();
        app.settings.needs_noita_reconciliation = true;
        app.noita_sync_state = NoitaSyncState::ReconciliationPending;

        app.save_detected_noita_as_preset("Recovered").unwrap();

        assert_eq!(mods::load_mod_config(&dir).unwrap()[0].name, "Detected");
        assert_eq!(app.selected_preset, "Recovered");
        assert_eq!(app.current_mods[0].name, "Detected");
        assert_eq!(app.noita_sync_state, NoitaSyncState::Live);
        assert!(!app.settings.needs_noita_reconciliation);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detected_setup_cannot_overwrite_an_existing_preset() {
        let dir = temp_config_dir("capture-duplicate-reconciliation");
        std::fs::create_dir_all(&dir).unwrap();
        let detected = vec![mod_entry("Detected", true, "1")];
        mods::write_mod_config(&dir, &mods::mods_to_xml(&detected)).unwrap();
        let (_runtime, mut app) = test_app(vec![mod_entry("Local", false, "2")]);
        app.settings.noita_dir = dir.to_string_lossy().to_string();
        app.settings.needs_noita_reconciliation = true;
        app.noita_sync_state = NoitaSyncState::ReconciliationPending;
        app.presets.insert(
            "Existing".to_string(),
            vec![mod_entry("Original", true, "3")],
        );

        let error = app
            .save_detected_noita_as_preset("Existing")
            .expect_err("an existing preset must not be overwritten");

        assert!(error.contains("already exists"));
        assert_eq!(app.presets["Existing"][0].name, "Original");
        assert_eq!(app.noita_sync_state, NoitaSyncState::ReconciliationPending);
        std::fs::remove_dir_all(&dir).ok();
    }
}
