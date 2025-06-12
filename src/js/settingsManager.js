import { state } from './state.js';

export class SettingsManager {
    constructor(modManager, uiManager) {
        this.modManager = modManager;
        this.uiManager = uiManager;
        this.settings = {
            noita_dir: '',
            entangled_dir: '',
            dark_mode: false
        };
    }

    async loadConfig() {
        const statusBar = document.getElementById('status-bar');
        if (!window.__TAURI__ || !window.__TAURI__.core) {
            statusBar.textContent = 'Tauri is not initialized';
            return;
        }

        try {
            let settings, presets;
            try {
                settings = await window.__TAURI__.core.invoke('load_settings');
                presets = await window.__TAURI__.core.invoke('load_presets');
            } catch (error) {
                console.error('Error loading config, using defaults:', error);
                settings = {
                    noita_dir: await window.__TAURI__.core.invoke('get_noita_save_path').catch(() => ''),
                    entangled_dir: '',
                    dark_mode: false
                };
                presets = {'Default': []};
                await this.saveDefaults(settings, presets);
                statusBar.textContent = 'Created default configuration';
            }

            this.applyConfig(settings, presets);
            if (settings.noita_dir) {
                await this.modManager.loadModConfigFromDirectory(settings.noita_dir);
            } else {
                statusBar.textContent = 'Noita save directory not set. Please set it in settings.';
            }
            console.log('Configuration loaded successfully');
        } catch (error) {
            console.error('Critical error in loadConfig:', error);
            statusBar.textContent = `Error loading configuration: ${error.message}`;
        }
    }

    async saveDefaults(settings, presets) {
        await window.__TAURI__.core.invoke('save_settings', { settings });
        await window.__TAURI__.core.invoke('save_presets', { presets });
    }

    applyConfig(settings, presets) {
        this.settings = settings;
        state.currentPresets = Object.keys(presets).reduce((acc, presetName) => {
            acc[presetName] = presets[presetName].map(mod => ({
                name: mod.name,
                enabled: mod.enabled,
                workshopId: mod.workshop_id,
                settingsFoldOpen: mod.settings_fold_open,
                index: 0
            }));
            return acc;
        }, {});
        state.isDarkMode = settings.dark_mode;
        document.getElementById('noita-dir').value = settings.noita_dir;
        document.getElementById('entangled-dir').value = settings.entangled_dir;
        document.getElementById('dark-mode-checkbox').checked = state.isDarkMode;
        this.uiManager.applyDarkMode();
    }

    async saveAndClose() {
        const statusBar = document.getElementById('status-bar');
        try {
            this.settings.noita_dir = document.getElementById('noita-dir').value;
            this.settings.entangled_dir = document.getElementById('entangled-dir').value;
            this.settings.dark_mode = state.isDarkMode;
            if (window.__TAURI__ && window.__TAURI__.core) {
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
                    await window.__TAURI__.core.invoke('save_settings', {settings: this.settings});
                    await window.__TAURI__.core.invoke('save_presets', {presets: presetsForSave});
                    statusBar.textContent = `Configuration saved successfully`;
                } catch (error) {
                    console.error('Save error:', error);
                    statusBar.textContent = `Error saving configuration: ${error.message}`;
                }
            }
            this.uiManager.changeView('main');
        } catch (error) {
            console.error('Save and close error:', error);
            statusBar.textContent = `Critical error during save: ${error.message}`;
            this.uiManager.changeView('main');
        }
    }

    async changeDirectory(type) {
        const statusBar = document.getElementById('status-bar');
        try {
            if (window.__TAURI__ && window.__TAURI__.dialog) {
                const selected = await window.__TAURI__.dialog.open({
                    directory: true,
                    multiple: false
                });
                if (selected) {
                    document.getElementById(type + '-dir').value = selected;
                    this.settings[type + '_dir'] = selected;
                    statusBar.className = 'status-bar';
                    statusBar.textContent = `Selected directory for ${type}: ${selected}`;
                    if (type === 'noita') {
                        await this.modManager.loadModConfigFromDirectory(selected);
                    }
                }
            }
        } catch (error) {
            console.error('Directory selection error:', error);
            statusBar.className = 'status-bar';
            statusBar.textContent = `Error selecting directory: ${error.message}`;
        }
    }

    async openDirectory(type) {
        console.log(`Open directory ${type} TBD`);
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
                console.error('Failed to get Noita save path:', error);
                statusBar.textContent = 'Unable to find Noita save directory. Please set it manually.';
                return; // Exit early to prevent invalid state
            }

            // Validate directory
            const configPath = `${defaultNoitaDir}/mod_config.xml`;
            const pathExists = await window.__TAURI__.core.invoke('read_mod_config', { directory: defaultNoitaDir })
                .then(() => true)
                .catch(() => false);
            if (!pathExists) {
                statusBar.textContent = 'Invalid Noita save directory. Please set a valid directory.';
                return; // Exit early
            }

            // Apply defaults
            this.settings.noita_dir = defaultNoitaDir;
            this.settings.entangled_dir = '';
            this.settings.dark_mode = false;
            state.isDarkMode = false;

            document.getElementById('noita-dir').value = defaultNoitaDir;
            document.getElementById('entangled-dir').value = '';
            document.getElementById('dark-mode-checkbox').checked = false;
            this.uiManager.applyDarkMode();

            // Reset presets to default
            state.currentPresets = { 'Default': [] };
            state.selectedPreset = 'Default';
            await window.__TAURI__.core.invoke('save_presets', { presets: { 'Default': [] } });

            // Reload mods
            await this.modManager.loadModConfigFromDirectory(defaultNoitaDir);

            // Save settings
            await window.__TAURI__.core.invoke('save_settings', { settings: this.settings });

            statusBar.textContent = 'Successfully reset to defaults';
        } catch (error) {
            console.error('Error resetting defaults:', error);
            statusBar.textContent = `Error resetting defaults: ${error.message}`;
        }
    }
}