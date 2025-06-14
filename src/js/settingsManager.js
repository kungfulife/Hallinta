import { state } from './state.js';

export class SettingsManager {
    constructor(modManager, uiManager) {
        this.modManager = modManager;
        this.uiManager = uiManager;
        this.settings = {
            noita_dir: '',
            entangled_dir: '',
            dark_mode: false,
            selected_preset: 'Default',
            version: '', // Version will be set by backend
            log_settings: {
                max_log_files: 50,
                max_log_size_mb: 10,
                log_level: 'INFO',
                auto_save: true
            }
        };
    }

    async loadConfig() {
        if (!window.__TAURI__?.core) {
            this.uiManager.showError('Tauri is not initialized. Application may not function correctly.');
            this.uiManager.logAction('ERROR', 'Tauri is not initialized');
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
                this.uiManager.showError(`Error loading configuration: ${error.message}. Using defaults.`);
                this.uiManager.logAction('ERROR', `Error loading config: ${error.message}`);
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
                    this.uiManager.showError('Could not determine default Noita save path.');
                    this.uiManager.logAction('ERROR', `Could not get default Noita path: ${pathError.message}`);
                    settings.noita_dir = '';
                }
                presets = { "Default": [] };
                await this.saveDefaults(settings, presets);
                this.uiManager.logAction('INFO', 'Created default configuration');
            }

            this.applyConfig(settings, presets);
            const versionElement = document.getElementById('app-version');
            if (versionElement) {
                versionElement.textContent = settings.version;
            }

            if (settings.noita_dir) {
                await this.modManager.loadModConfigFromDirectory(settings.noita_dir);
            } else {
                this.uiManager.showError('Noita save directory not set. Please set it in settings.');
                this.uiManager.logAction('ERROR', 'Noita save directory not set. Please set it in settings.');
                hasError = true;
            }

            if (!hasError) {
                this.uiManager.logAction('INFO', 'Configuration loaded successfully');
            }
            return !hasError;
        } catch (error) {
            this.uiManager.showError(`Critical error in loadConfig: ${error.message}`);
            this.uiManager.logAction('ERROR', `Critical error in loadConfig: ${error.message}`);
            return false;
        }
    }

    async saveDefaults(settings, presets) {
        try {
            await window.__TAURI__.core.invoke('save_settings', { settings });
            await window.__TAURI__.core.invoke('save_presets', { presets });
        } catch (error) {
            this.uiManager.showError(`Error saving default settings: ${error.message}`);
            this.uiManager.logAction('ERROR', `Error saving default settings: ${error.message}`);
        }
    }

    applyConfig(settings, presets) {
        this.settings = settings;

        // Apply presets
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

        // Set selected preset
        state.selectedPreset = settings.selected_preset || 'Default';

        // Apply other settings
        state.isDarkMode = settings.dark_mode;

        const noitaDirElement = document.getElementById('noita-dir');
        const entangledDirElement = document.getElementById('entangled-dir');
        const darkModeElement = document.getElementById('dark-mode-checkbox');

        if (noitaDirElement) noitaDirElement.value = settings.noita_dir;
        if (entangledDirElement) entangledDirElement.value = settings.entangled_dir;
        if (darkModeElement) darkModeElement.checked = state.isDarkMode;

        this.uiManager.applyDarkMode();
    }

    async saveAndClose() {
        try {
            const noitaDirElement = document.getElementById('noita-dir');
            const entangledDirElement = document.getElementById('entangled-dir');
            this.settings.noita_dir = noitaDirElement ? noitaDirElement.value : '';
            this.settings.entangled_dir = entangledDirElement ? entangledDirElement.value : '';
            this.settings.dark_mode = state.isDarkMode;
            this.settings.selected_preset = state.selectedPreset;
            this.settings.version = await window.__TAURI__.core.invoke('get_version').catch(() => 'unknown');

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
                this.uiManager.logAction('INFO', 'Configuration saved successfully');
            }

            this.uiManager.changeView('main');
        } catch (error) {
            this.uiManager.showError(`Critical error during save: ${error.message}`);
            this.uiManager.logAction('ERROR', `Critical error during save: ${error.message}`);
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
                    this.uiManager.logAction('INFO', `Selected directory for ${type}: ${selected}`);
                    if (type === 'noita') {
                        await this.modManager.loadModConfigFromDirectory(selected);
                    }
                }
            }
        } catch (error) {
            this.uiManager.showError(`Error selecting directory: ${error.message}`);
            this.uiManager.logAction('ERROR', `Error selecting directory: ${error.message}`);
        }
    }

    async openDirectory(type) {
        const dirElement = document.getElementById(`${type}-dir`);
        const directory = dirElement ? dirElement.value : '';
        if (!directory) {
            this.uiManager.showError(`No ${type} directory set`);
            this.uiManager.logAction('ERROR', `No ${type} directory set`);
            return;
        }

        try {
            await window.__TAURI__.core.invoke('open_directory', { directory });
            this.uiManager.logAction('INFO', `Opened ${type} directory: ${directory}`);
        } catch (error) {
            this.uiManager.showError(`Error opening directory: ${error.message}`);
            this.uiManager.logAction('ERROR', `Error opening directory: ${error.message}`);
        }
    }

    async openAppSettingsFolder() {
        try {
            const settingsDir = await window.__TAURI__.core.invoke('get_app_settings_dir');
            await window.__TAURI__.core.invoke('open_directory', { directory: settingsDir });
            this.uiManager.logAction('INFO', `Opened directory: ${settingsDir}`);
        } catch (error) {
            this.uiManager.showError(`Error opening directory: ${error.message}`);
            this.uiManager.logAction('ERROR', `Error opening directory: ${error.message}`);
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
                this.uiManager.showError('Failed to get Noita save path. Please set manually.');
                this.uiManager.logAction('ERROR', 'Failed to get Noita save path');
                this.uiManager.logAction('ERROR', 'Unable to find directory. Please set manually.');
                return;
            }
            const pathExists = await window.__TAURI__.core.invoke('read_mod_config', { directory: defaultNoitaDir })
                .then(() => true)
                .catch(() => false);
            if (!pathExists) {
                this.uiManager.showError('Invalid Noita save directory. Please set a valid directory.');
                this.uiManager.logAction('ERROR', 'Invalid Noita save directory');
                this.uiManager.logAction('ERROR', 'Invalid directory. Please set a valid directory.');
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
            const noitaDirElement = document.getElementById('noita-dir');
            const entangledDirElement = document.getElementById('entangled-dir');
            const darkModeElement = document.getElementById('dark-mode-checkbox');
            if (noitaDirElement) noitaDirElement.value = defaultNoitaDir;
            if (entangledDirElement) entangledDirElement.value = '';
            if (darkModeElement) darkModeElement.checked = false;
            this.uiManager.applyDarkMode();
            state.currentPresets = { "Default": [] };
            await window.__TAURI__.core.invoke('save_presets', { presets: { "Default": [] } });
            await this.modManager.loadModConfigFromDirectory(defaultNoitaDir);
            await window.__TAURI__.core.invoke('save_settings', { settings: this.settings });
            this.uiManager.logAction('INFO', 'Successfully reset to defaults');
        } catch (error) {
            this.uiManager.showError(`Error resetting defaults: ${error.message}`);
            this.uiManager.logAction('ERROR', `Error resetting defaults: ${error.message}`);
        }
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'SettingsManager');
    }
}