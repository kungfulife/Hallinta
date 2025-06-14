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
            version: '0.3.0',
            log_settings: {
                max_log_files: 50,
                max_log_size_mb: 10,
                log_level: 'INFO',
                auto_save: true
            }
        };
    }

    async loadConfig() {
        const statusBar = document.getElementById('status-bar');

        if (!window.__TAURI__?.core) {
            this.uiManager.showError('Tauri is not initialized');
            this.logAction('ERROR', 'Tauri is not initialized');
            return;
        }

        try {
            let settings, presets;

            try {
                settings = await window.__TAURI__.core.invoke('load_settings');
                presets = await window.__TAURI__.core.invoke('load_presets');
            } catch (error) {
                this.uiManager.showError(`Error loading config, using defaults: ${error.message}`);
                this.logAction('ERROR', `Error loading config: ${error.message}`);

                // Create proper default settings structure
                settings = {
                    noita_dir: '',
                    entangled_dir: '',
                    dark_mode: false,
                    selected_preset: 'Default',
                    version: '0.3.0',
                    log_settings: {
                        max_log_files: 50,
                        max_log_size_mb: 10,
                        log_level: 'INFO',
                        auto_save: true
                    }
                };

                // Try to get default Noita path
                try {
                    settings.noita_dir = await window.__TAURI__.core.invoke('get_noita_save_path');
                } catch (pathError) {
                    this.uiManager.showError(`Could not get default Noita path: ${pathError.message}`);
                    this.logAction('ERROR', `Could not get default Noita path: ${pathError.message}`);
                    settings.noita_dir = '';
                }

                presets = { "Default": [] };
                await this.saveDefaults(settings, presets);
                statusBar.textContent = 'Created default configuration';
            }

            this.applyConfig(settings, presets);

            // Display version
            const versionElement = document.getElementById('app-version');
            if (versionElement) {
                versionElement.textContent = settings.version;
            }

            if (settings.noita_dir) {
                await this.modManager.loadModConfigFromDirectory(settings.noita_dir);
            } else {
                statusBar.textContent = 'Noita save directory not set. Please set it in settings.';
                this.logAction('WARN', 'Noita save directory not set');
            }

            this.logAction('INFO', 'Configuration loaded successfully');

        } catch (error) {
            this.uiManager.showError(`Critical error in loadConfig: ${error.message}`);
            this.logAction('ERROR', `Critical error in loadConfig: ${error.message}`);
        }
    }

    async saveDefaults(settings, presets) {
        try {
            await window.__TAURI__.core.invoke('save_settings', { settings });
            await window.__TAURI__.core.invoke('save_presets', { presets });
        } catch (error) {
            this.uiManager.showError(`Error saving default settings: ${error.message}`);
            this.logAction('ERROR', `Error saving default settings: ${error.message}`);
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
        const statusBar = document.getElementById('status-bar');

        try {
            const noitaDirElement = document.getElementById('noita-dir');
            const entangledDirElement = document.getElementById('entangled-dir');

            this.settings.noita_dir = noitaDirElement ? noitaDirElement.value : '';
            this.settings.entangled_dir = entangledDirElement ? entangledDirElement.value : '';
            this.settings.dark_mode = state.isDarkMode;
            this.settings.selected_preset = state.selectedPreset;

            if (window.__TAURI__?.core) {
                // Prepare presets for save
                const presetsForSave = {};
                Object.keys(state.currentPresets).forEach(presetName => {
                    presetsForSave[presetName] = state.currentPresets[presetName].map(mod => ({
                        name: mod.name,
                        enabled: mod.enabled,
                        workshop_id: mod.workshopId || '0',
                        settings_fold_open: mod.settingsFoldOpen || false
                    }));
                });

                try {
                    await window.__TAURI__.core.invoke('save_settings', { settings: this.settings });
                    await window.__TAURI__.core.invoke('save_presets', { presets: presetsForSave });
                    statusBar.textContent = 'Configuration saved successfully';
                    this.logAction('INFO', 'Configuration saved successfully');
                } catch (error) {
                    this.uiManager.showError(`Error saving configuration: ${error.message}`);
                    this.logAction('ERROR', `Error saving configuration: ${error.message}`);
                }
            }

            this.uiManager.changeView('main');

        } catch (error) {
            this.uiManager.showError(`Critical error during save: ${error.message}`);
            this.logAction('ERROR', `Critical error during save: ${error.message}`);
            this.uiManager.changeView('main');
        }
    }

    async changeDirectory(type) {
        const statusBar = document.getElementById('status-bar');

        try {
            if (window.__TAURI__?.dialog) {
                const selected = await window.__TAURI__.dialog.open({
                    directory: true,
                    multiple: false
                });

                if (selected) {
                    const dirElement = document.getElementById(`${type}-dir`);
                    if (dirElement) {
                        dirElement.value = selected;
                    }
                    this.settings[`${type}_dir`] = selected;
                    statusBar.className = 'status-bar';
                    statusBar.textContent = `Selected directory for ${type}: ${selected}`;

                    // If it's the noita directory, reload the mod config
                    if (type === 'noita') {
                        await this.modManager.loadModConfigFromDirectory(selected);
                    }

                    this.logAction('INFO', `Changed ${type} directory to ${selected}`);
                }
            }
        } catch (error) {
            this.uiManager.showError(`Error selecting directory: ${error.message}`);
            this.logAction('ERROR', `Error selecting directory: ${error.message}`);
        }
    }

    async openDirectory(type) {
        const dirElement = document.getElementById(`${type}-dir`);
        const directory = dirElement ? dirElement.value : '';

        if (!directory) {
            this.uiManager.showError(`No ${type} directory set`);
            this.logAction('ERROR', `No ${type} directory set`);
            return;
        }

        try {
            await window.__TAURI__.core.invoke('open_directory', { directory });
            document.getElementById('status-bar').textContent = `Opened ${type} directory`;
            this.logAction('INFO', `Opened ${type} directory: ${directory}`);
        } catch (error) {
            this.uiManager.showError(`Error opening directory: ${error.message}`);
            this.logAction('ERROR', `Error opening directory: ${error.message}`);
        }
    }

    async openAppSettingsFolder() {
        try {
            const settingsDir = await window.__TAURI__.core.invoke('get_app_settings_dir');
            await window.__TAURI__.core.invoke('open_directory', { directory: settingsDir });
            document.getElementById('status-bar').textContent = 'Opened application settings folder';
            this.logAction('INFO', `Opened application settings folder: ${settingsDir}`);
        } catch (error) {
            this.uiManager.showError(`Error opening app settings folder: ${error.message}`);
            this.logAction('ERROR', `Error opening app settings folder: ${error.message}`);
        }
    }

    async resetToDefaults() {
        const statusBar = document.getElementById('status-bar');

        try {
            if (!window.__TAURI__?.core) {
                throw new Error('Tauri is not initialized');
            }

            let defaultNoitaDir;
            try {
                defaultNoitaDir = await window.__TAURI__.core.invoke('get_noita_save_path');
            } catch (error) {
                this.uiManager.showError(`Failed to get Noita save path: ${error.message}`);
                this.logAction('ERROR', `Failed to get Noita save path: ${error.message}`);
                statusBar.textContent = 'Unable to find Noita save directory. Please set it manually.';
                return;
            }

            // Validate directory
            const pathExists = await window.__TAURI__.core.invoke('read_mod_config', { directory: defaultNoitaDir })
                .then(() => true)
                .catch(() => false);

            if (!pathExists) {
                this.uiManager.showError('Invalid Noita save directory');
                this.logAction('ERROR', 'Invalid Noita save directory');
                statusBar.textContent = 'Invalid Noita save directory. Please set a valid directory.';
                return;
            }

            // Apply defaults with proper structure
            this.settings = {
                noita_dir: defaultNoitaDir,
                entangled_dir: '',
                dark_mode: false,
                selected_preset: 'Default',
                version: '0.3.0',
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

            // Reset presets to default
            state.currentPresets = { "Default": [] };
            await window.__TAURI__.core.invoke('save_presets', { presets: { "Default": [] } });

            // Reload mods
            await this.modManager.loadModConfigFromDirectory(defaultNoitaDir);

            // Save settings
            await window.__TAURI__.core.invoke('save_settings', { settings: this.settings });

            statusBar.textContent = 'Successfully reset to defaults';
            this.logAction('INFO', 'Successfully reset to defaults');

        } catch (error) {
            this.uiManager.showError(`Error resetting defaults: ${error.message}`);
            this.logAction('ERROR', `Error resetting defaults: ${error.message}`);
        }
    }

    logAction(level, message) {
        if (window.__TAURI__ && window.__TAURI__.core) {
            window.__TAURI__.core.invoke('add_log_entry', {
                level,
                message,
                module: 'SettingsManager'
            }).catch(error => {
                this.uiManager.showError(`Failed to log action: ${error.message}`);
            });
            if (level === 'ERROR') {
                this.uiManager.showError(message);
            }
        }
    }
}