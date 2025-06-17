import { state } from './state.js';
import { deepCopyMods } from './state.js';
export class SettingsManager {
    constructor(modManager, uiManager) {
        this.modManager = modManager;
        this.uiManager = uiManager;
        this.settings = {
            noita_dir: '',
            entangled_dir: '',
            dark_mode: false,
            selected_preset: 'Default',
            version: '',
            log_settings: {
                max_log_files: 50,
                max_log_size_mb: 10,
                log_level: 'INFO',
                auto_save: true
            }
        };
        this.previousSettings = null;
        this.previousIsDarkMode = false;
    }

    async loadConfig() {
        if (!window.__TAURI__?.core) {
            this.logAction('ERROR', 'Tauri is not initialized. Application may not function correctly.')
            return false;
        }

        let hasError = false;
        try {
            let settings, presets, version;
            try {
                version = await window.__TAURI__.core.invoke('get_version');
                settings = await window.__TAURI__.core.invoke('load_settings');
                presets = await window.__TAURI__.core.invoke('load_presets');
            } catch (error) {
                this.logAction('ERROR', `Error loading configuration: ${error.message}. Using defaults.`)
                this.logAction('ERROR', `Error loading config: ${error.message}`);
                hasError = true;
                settings = {
                    noita_dir: '',
                    entangled_dir: '',
                    dark_mode: false,
                    selected_preset: 'Default',
                    version: version || 'unknown',
                    log_settings: {
                        max_log_files: 50,
                        max_log_size_mb: 10,
                        log_level: 'INFO',
                        auto_save: true
                    }
                };
                try {
                    settings.noita_dir = await window.__TAURI__.core.invoke('get_noita_save_path');
                } catch (pathError) {
                    this.logAction('ERROR', `Could not get default Noita path: ${pathError.message}`);
                    settings.noita_dir = '';
                }
                presets = { "Default": [] };
                await this.saveDefaults(settings, presets);
                this.logAction('INFO', 'Created default configuration');
            }

            this.applyConfig(settings, presets);
            const versionElement = document.getElementById('app-version');
            if (versionElement) {
                versionElement.textContent = settings.version;
            }

            if (settings.noita_dir) {
                const configPath = `${settings.noita_dir}/mod_config.xml`;
                const fileExists = await window.__TAURI__.core.invoke('check_file_exists', { path: configPath });
                if (fileExists) {
                    await this.modManager.loadModConfigFromDirectory(settings.noita_dir);
                } else {
                    this.logAction('ERROR', 'Noita save directory does not contain mod_config.xml');
                    hasError = true;
                }
            } else {
                this.logAction('ERROR', 'Noita save directory not set. Please set it in settings.');
                hasError = true;
            }

            if (!hasError) {
                this.logAction('INFO', 'Configuration loaded successfully');
            }
            return !hasError;
        } catch (error) {
            this.logAction('ERROR', `Critical error in loadConfig: ${error.message}`);
            return false;
        }
    }

    async saveDefaults(settings, presets) {
        try {
            await window.__TAURI__.core.invoke('save_settings', { settings });
            await window.__TAURI__.core.invoke('save_presets', { presets });
        } catch (error) {
            this.logAction('ERROR', `Error saving default settings: ${error.message}`);
        }
    }

    applyConfig(settings, presets) {
        this.settings = settings;
        state.currentPresets = Object.keys(presets).reduce((acc, presetName) => {
            acc[presetName] = presets[presetName].map(mod => ({
                name: mod.name,
                enabled: mod.enabled,
                workshopId: mod.workshop_id || '0',
                settingsFoldOpen: mod.settings_fold_open || false,
                index: 0
            }));
            return acc;
        }, {});
        state.selectedPreset = settings.selected_preset || 'Default';
        if (!state.currentPresets[state.selectedPreset]) {
            state.selectedPreset = 'Default';
            if (!state.currentPresets['Default']) {
                state.currentPresets['Default'] = [];
            }
        }

        state.isDarkMode = settings.dark_mode;
        const noitaDirElement = document.getElementById('noita-dir');
        const entangledDirElement = document.getElementById('entangled-dir');
        const darkModeElement = document.getElementById('dark-mode-checkbox');
        const logLevelSelect = document.getElementById('log-level-select');

        if (noitaDirElement) noitaDirElement.value = settings.noita_dir;
        if (entangledDirElement) entangledDirElement.value = settings.entangled_dir;
        if (darkModeElement) darkModeElement.checked = state.isDarkMode;
        if (logLevelSelect) logLevelSelect.value = settings.log_settings.log_level || 'INFO';

        this.uiManager.applyDarkMode();
    }

    async saveAndClose() {
        try {
            const noitaDirElement = document.getElementById('noita-dir');
            const entangledDirElement = document.getElementById('entangled-dir');
            const logLevelSelect = document.getElementById('log-level-select');
            this.settings.noita_dir = noitaDirElement ? noitaDirElement.value : '';
            this.settings.entangled_dir = entangledDirElement ? entangledDirElement.value : '';
            this.settings.dark_mode = state.isDarkMode;
            this.settings.selected_preset = state.selectedPreset;
            if (logLevelSelect) this.settings.log_settings.log_level = logLevelSelect.value;
            this.settings.version = await window.__TAURI__.core.invoke('get_version').catch(() => 'unknown');

            if (this.settings.noita_dir) {
                const configPath = `${this.settings.noita_dir}/mod_config.xml`;
                const fileExists = await window.__TAURI__.core.invoke('check_file_exists', { path: configPath });
                if (!fileExists) {
                    this.logAction('ERROR', 'Cannot save: mod_config.xml not found in Noita directory');
                    return;
                }
            }

            // TOOD: Semi duplicate code
            if (window.__TAURI__?.core) {
                const presetsForSave = {};
                Object.keys(state.currentPresets).forEach(presetName => {
                    presetsForSave[presetName] = state.currentPresets[presetName].map(mod => ({
                        name: mod.name,
                        enabled: mod.enabled,
                        workshop_id: mod.workshopId || '0',
                        settings_fold_open: mod.settingsFoldOpen || false
                    }));
                });
                await window.__TAURI__.core.invoke('save_settings', { settings: this.settings });
                await window.__TAURI__.core.invoke('save_presets', { presets: presetsForSave });
                await this.modManager.saveModConfigToFile();
                this.logAction('INFO', 'Configuration saved successfully');
            }

            this.uiManager.changeView('main');
        } catch (error) {
            this.logAction('ERROR', `Critical error during save: ${error.message}`);
            this.uiManager.changeView('main');
        }
    }

    async changeDirectory(type) {
        try {
            if (window.__TAURI__?.dialog) {
                const selected = await window.__TAURI__.dialog.open({
                    directory: '',
                    multiple: false
                });
                if (selected) {
                    const dirElement = document.getElementById(`${type}-dir`);
                    if (dirElement) dirElement.value = selected;
                    this.settings[`${type}_dir`] = selected;
                    this.logAction('DEBUG', `Selected directory for ${type}: ${selected}`);
                    if (type === 'noita') {
                        await this.modManager.loadModConfigFromDirectory(selected);
                    }
                }
            }
        } catch (error) {
            this.logAction('ERROR', `Error selecting directory: ${error.message}`);
        }
    }

    async openDirectory(type) {
        const dirElement = document.getElementById(`${type}-dir`);
        const directory = dirElement ? dirElement.value : '';
        if (!directory) {
            this.logAction('ERROR', `No ${type} directory set`);
            return;
        }

        try {
            await window.__TAURI__.core.invoke('open_directory', { directory });
            this.logAction('DEBUG', `Opened ${type} directory: ${directory}`);
        } catch (error) {
            this.logAction('ERROR', `Error opening directory: ${error.message}`);
        }
    }

    async openAppSettingsFolder() {
        try {
            const settingsDir = await window.__TAURI__.core.invoke('get_app_settings_dir');
            await window.__TAURI__.core.invoke('open_directory', { directory: settingsDir });
            this.logAction('DEBUG', `Opened directory: ${settingsDir}`);
        } catch (error) {
            this.logAction('ERROR', `Error opening directory: ${error.message}`);
        }
    }

    async resetToDefaults() {
        try {
            if (!window.__TAURI__?.core) {
                throw new Error('Tauri is not initialized');
            }
            let defaultNoitaDir;
            try {
                defaultNoitaDir = await window.__TAURI__.core.invoke('get_noita_save_path');
            } catch {
                this.logAction('ERROR', 'Failed to get Noita save path');
                return;
            }
            this.settings = {
                noita_dir: defaultNoitaDir,
                entangled_dir: '',
                dark_mode: false,
                selected_preset: 'Default',
                version: await window.__TAURI__.core.invoke('get_version').catch(() => 'unknown'),
                log_settings: {
                    max_log_files: 50,
                    max_log_size_mb: 10,
                    log_level: 'INFO',
                    auto_save: true
                }
            };
            state.isDarkMode = false;
            state.selectedPreset = 'Default';
            if (!state.currentPresets['Default']) {
                state.currentPresets['Default'] = [];
            }
            state.currentMods = deepCopyMods(state.currentPresets['Default']);
            const noitaDirElement = document.getElementById('noita-dir');
            const entangledDirElement = document.getElementById('entangled-dir');
            const darkModeElement = document.getElementById('dark-mode-checkbox');
            const logLevelSelect = document.getElementById('log-level-select');
            if (noitaDirElement) noitaDirElement.value = defaultNoitaDir;
            if (entangledDirElement) entangledDirElement.value = '';
            if (darkModeElement) darkModeElement.checked = false;
            if (logLevelSelect) logLevelSelect.value = 'INFO';
            this.uiManager.applyDarkMode();
            await this.modManager.loadModConfigFromDirectory(defaultNoitaDir);
            this.logAction('INFO', 'Settings reset to defaults. Press Save & Close to apply.');
        } catch (error) {
            this.logAction('ERROR', `Error resetting defaults: ${error.message}`);
        }
    }

    storeCurrentSettings() {
        this.previousSettings = { ...this.settings };
        this.previousIsDarkMode = state.isDarkMode;
    }

    restorePreviousSettings() {
        if (this.previousSettings) {
            this.settings = { ...this.previousSettings };
            state.isDarkMode = this.previousIsDarkMode;
            const noitaDirElement = document.getElementById('noita-dir');
            const entangledDirElement = document.getElementById('entangled-dir');
            const darkModeElement = document.getElementById('dark-mode-checkbox');
            const logLevelSelect = document.getElementById('log-level-select');
            if (noitaDirElement) noitaDirElement.value = this.settings.noita_dir;
            if (entangledDirElement) entangledDirElement.value = this.settings.entangled_dir;
            if (darkModeElement) darkModeElement.checked = state.isDarkMode;
            if (logLevelSelect) logLevelSelect.value = this.settings.log_settings.log_level;
            this.uiManager.applyDarkMode();

            this.logAction('DEBUG', 'Restored Previous Settings');
        }
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'SettingsManager');
    }
}