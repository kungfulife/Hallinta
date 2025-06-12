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
        try {
            if (window.__TAURI__ && window.__TAURI__.core) {
                try {
                    const settings = await window.__TAURI__.core.invoke('load_settings');
                    const presets = await window.__TAURI__.core.invoke('load_presets');
                    this.settings = settings;
                    state.currentPresets = {};
                    Object.keys(presets).forEach(presetName => {
                        state.currentPresets[presetName] = presets[presetName].map(mod => ({
                            name: mod.name,
                            enabled: mod.enabled,
                            workshopId: mod.workshop_id,
                            settingsFoldOpen: mod.settings_fold_open,
                            index: 0
                        }));
                    });
                    state.isDarkMode = settings.dark_mode;
                    document.getElementById('noita-dir').value = settings.noita_dir;
                    document.getElementById('entangled-dir').value = settings.entangled_dir;
                    document.getElementById('dark-mode-checkbox').checked = state.isDarkMode;
                    this.uiManager.applyDarkMode();
                    if (settings.noita_dir) {
                        await this.modManager.loadModConfigFromDirectory(settings.noita_dir);
                    }
                    console.log('Configuration loaded successfully');
                } catch (error) {
                    console.error('Error loading from files, creating defaults:', error);
                    const defaultSettings = {
                        noita_dir: await window.__TAURI__.core.invoke('get_noita_save_path'),
                        entangled_dir: '',
                        dark_mode: false
                    };
                    const defaultPresets = {'Default': []};
                    try {
                        await window.__TAURI__.core.invoke('save_settings', {settings: defaultSettings});
                        await window.__TAURI__.core.invoke('save_presets', {presets: defaultPresets});
                        this.settings = defaultSettings;
                        state.currentPresets = defaultPresets;
                        state.isDarkMode = defaultSettings.dark_mode;
                        document.getElementById('noita-dir').value = defaultSettings.noita_dir;
                        document.getElementById('entangled-dir').value = defaultSettings.entangled_dir;
                        document.getElementById('dark-mode-checkbox').checked = state.isDarkMode;
                        await this.modManager.loadModConfigFromDirectory(defaultSettings.noita_dir);
                        document.getElementById('status-bar').textContent = 'Created default configuration';
                    } catch (saveError) {
                        console.error('Error creating defaults:', saveError);
                    }
                }
            }
        } catch (error) {
            console.error('Error loading configuration:', error);
        }
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
        // Placeholder for opening directory in file explorer (TBD)
        console.log(`Open directory ${type} TBD`);
    }

    async resetToDefaults() {
        const statusBar = document.getElementById('status-bar');
        try {
            if (!window.__TAURI__ || !window.__TAURI__.core) {
                console.error('Tauri is not initialized');
                statusBar.textContent = `Error resetting defaults: Tauri is not initialized`;
                return;
            }
            const defaultNoitaDir = await window.__TAURI__.core.invoke('get_noita_save_path');
            document.getElementById('noita-dir').value = defaultNoitaDir;
            document.getElementById('entangled-dir').value = '';
            this.settings.noita_dir = defaultNoitaDir;
            this.settings.entangled_dir = '';
            this.settings.dark_mode = false;
            state.isDarkMode = false;
            document.getElementById('dark-mode-checkbox').checked = false;
            this.uiManager.applyDarkMode();
            await this.modManager.loadModConfigFromDirectory(defaultNoitaDir);
            statusBar.textContent = 'Successfully reset to defaults';
        } catch (error) {
            console.error('Reset defaults error:', error);
            statusBar.textContent = `Error resetting defaults: ${error.message}`;
        }
    }
}