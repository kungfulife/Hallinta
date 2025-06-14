import { state } from './state.js';

export class PresetManager {
    constructor(uiManager, modManager) {
        this.uiManager = uiManager;
        this.modManager = modManager;
    }

    loadPresets() {
        const selector = document.getElementById('preset-dropdown');
        if (!selector) {
            this.logAction('ERROR', 'Preset dropdown element not found');
            return;
        }
        selector.innerHTML = '';

        const createOption = document.createElement('option');
        createOption.value = 'createnew';
        createOption.textContent = 'Create New Preset';
        selector.appendChild(createOption);

        Object.keys(state.currentPresets).forEach(preset => {
            const option = document.createElement('option');
            option.value = preset;
            option.textContent = preset;
            if (preset === state.selectedPreset) {
                option.selected = true;
            }
            selector.appendChild(option);
        });
    }

    async onPresetChange() {
        const selector = document.getElementById('preset-dropdown');
        if (!selector) {
            this.logAction('ERROR', 'Preset dropdown element not found');
            return;
        }
        const selectedValue = selector.value;

        if (!this.modManager) {
            this.logAction('ERROR', 'ModManager is not initialized');
            selector.value = state.selectedPreset;
            return;
        }

        try {
            if (selectedValue === 'createnew') {
                const newName = prompt('Enter name for new preset:', `Preset ${Object.keys(state.currentPresets).length + 1}`);
                if (newName && newName.trim() !== '' && !state.currentPresets[newName]) {
                    state.currentPresets[newName] = [...state.currentMods];
                    state.selectedPreset = newName;
                    await this.saveSelectedPreset();
                    await this.modManager.saveModConfigToFile();
                    this.loadPresets();
                    this.uiManager.logAction('INFO', `Created new preset: ${newName}`);
                } else {
                    selector.value = state.selectedPreset;
                    if (newName === null) {
                        this.uiManager.logAction('INFO', 'Preset creation canceled');
                    } else {
                        this.uiManager.logAction('ERROR', 'Invalid preset name or preset already exists');
                    }
                }
            } else if (state.currentPresets[selectedValue] && Array.isArray(state.currentPresets[selectedValue])) {
                state.selectedPreset = selectedValue;
                state.currentMods = [...state.currentPresets[selectedValue]];
                await this.modManager.saveModConfigToFile();
                await this.saveSelectedPreset();
                this.uiManager.renderModList();
                this.uiManager.updateModCount();
                this.uiManager.logAction('INFO', `Switched to preset: ${selectedValue}`);
            } else {
                this.logAction('ERROR', `Preset ${selectedValue} is invalid or not found`);
                selector.value = state.selectedPreset;
            }
        } catch (error) {
            this.logAction('ERROR', `Error changing preset: ${error.message}`);
            selector.value = state.selectedPreset;
        }
    }

    async deleteCurrentPreset() {
        if (state.selectedPreset === 'Default') {
            this.logAction('WARN', 'Cannot delete default preset');
            return;
        }

        if (!this.modManager) {
            this.logAction('ERROR', 'ModManager is not initialized');
            return;
        }

        try {
            if (window.confirm(`Delete preset "${state.selectedPreset}"?`)) {
                const deletedPreset = state.selectedPreset;
                delete state.currentPresets[state.selectedPreset];
                state.selectedPreset = 'Default';
                state.currentMods = [...(state.currentPresets['Default'] || [])];
                await this.modManager.saveModConfigToFile();
                await this.saveSelectedPreset();
                this.uiManager.renderModList();
                this.uiManager.updateModCount();
                this.loadPresets();
                this.logAction('INFO', `Deleted preset: ${deletedPreset}`);
            }
        } catch (error) {
            this.logAction('ERROR', `Error deleting preset: ${error.message}`);
        }
    }

    async renameCurrentPreset() {
        if (state.selectedPreset === 'Default') {
            this.logAction('ERROR', 'Cannot rename default preset');
            return;
        }

        if (!this.modManager) {
            this.logAction('ERROR', 'ModManager is not initialized');
            return;
        }

        try {
            const newName = prompt('Enter new name:', state.selectedPreset);
            if (newName && newName.trim() !== '' && newName !== state.selectedPreset && !state.currentPresets[newName]) {
                const oldName = state.selectedPreset;
                state.currentPresets[newName] = state.currentPresets[state.selectedPreset];
                delete state.currentPresets[state.selectedPreset];
                state.selectedPreset = newName;
                await this.saveSelectedPreset();
                await this.modManager.saveModConfigToFile();
                this.loadPresets();
                this.logAction('INFO', `Renamed preset from ${oldName} to ${newName}`);
            } else {
                if (newName === null) {
                    this.logAction('INFO', 'Preset rename canceled');
                } else {
                    this.logAction('ERROR', 'Invalid preset name or preset already exists');
                }
            }
        } catch (error) {
            this.logAction('ERROR', `Error renaming preset: ${error.message}`);
        }
    }

    async saveSelectedPreset() {
        try {
            if (window.__TAURI__ && window.__TAURI__.core) {
                const settings = await window.__TAURI__.core.invoke('load_settings');
                settings.selected_preset = state.selectedPreset || 'Default';
                const presetsForSave = {};
                Object.keys(state.currentPresets).forEach(presetName => {
                    presetsForSave[presetName] = state.currentPresets[presetName].map(mod => ({
                        name: mod.name,
                        enabled: mod.enabled,
                        workshop_id: mod.workshopId || '0',
                        settings_fold_open: mod.settingsFoldOpen || false
                    }));
                });
                await window.__TAURI__.core.invoke('save_settings', { settings });
                await window.__TAURI__.core.invoke('save_presets', { presets: presetsForSave });
                this.logAction('INFO', `Saved preset: ${state.selectedPreset}`);
            }
        } catch (error) {
            this.logAction('ERROR', `Failed to save selected preset: ${error.message}`);
            throw error;
        }
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'PresetManager');
    }
}