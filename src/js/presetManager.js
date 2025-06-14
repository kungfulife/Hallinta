import { state } from './state.js';

export class PresetManager {
    constructor(uiManager) {
        this.uiManager = uiManager;
    }

    loadPresets() {
        const selector = document.getElementById('preset-dropdown');
        selector.innerHTML = '';

        // Create New Preset option
        const createOption = document.createElement('option');
        createOption.value = 'createnew';
        createOption.textContent = 'Create New Preset';
        selector.appendChild(createOption);

        // Load existing presets
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
        if (selector.value === 'createnew') {
            const newName = prompt('Enter name for new preset:', `Preset ${Object.keys(state.currentPresets).length + 1}`);
            if (newName && !state.currentPresets[newName]) {
                state.currentPresets[newName] = [...state.currentMods];
                state.selectedPreset = newName;
                this.loadPresets();
                await this.saveSelectedPreset();
                this.uiManager.logAction('INFO', `Created new preset: ${newName}`);
            } else {
                selector.value = state.selectedPreset;
                this.uiManager.logAction('ERROR', 'Invalid preset name or preset already exists');
            }
        } else {
            state.selectedPreset = selector.value;
            state.currentMods = [...state.currentPresets[state.selectedPreset]];
            this.uiManager.renderModList();
            this.uiManager.updateModCount();
            await this.saveSelectedPreset();
            this.uiManager.logAction('INFO', `Switched to preset: ${state.selectedPreset}`);
        }
    }

    async deleteCurrentPreset() {
        if (state.selectedPreset === 'Default') {
            this.uiManager.logAction('WARN', 'Cannot delete the Default preset');
            return;
        }

        if (window.confirm(`Delete preset "${state.selectedPreset}"?`)) {
            const deletedPreset = state.selectedPreset;
            delete state.currentPresets[state.selectedPreset];
            state.selectedPreset = 'Default';
            state.currentMods = [...state.currentPresets[state.selectedPreset]];
            this.uiManager.renderModList();
            this.uiManager.updateModCount();
            this.loadPresets();
            await this.saveSelectedPreset();
            this.uiManager.logAction('INFO', `Deleted preset: ${deletedPreset}`);
        }
    }

    async renameCurrentPreset() {
        if (state.selectedPreset === 'Default') {
            this.uiManager.logAction('WARN', 'Cannot rename the Default preset');
            return;
        }

        const newName = prompt('Enter new name:', state.selectedPreset);
        if (newName && newName !== state.selectedPreset && !state.currentPresets[newName]) {
            const oldName = state.selectedPreset;
            state.currentPresets[newName] = state.currentPresets[state.selectedPreset];
            delete state.currentPresets[state.selectedPreset];
            state.selectedPreset = newName;
            this.loadPresets();
            await this.saveSelectedPreset();
            this.uiManager.logAction('INFO', `Renamed preset from ${oldName} to ${newName}`);
        } else {
            this.uiManager.logAction('ERROR', 'Invalid preset name or preset already exists');
        }
    }

    async saveSelectedPreset() {
        try {
            if (window.__TAURI__ && window.__TAURI__.core) {
                const settings = await window.__TAURI__.core.invoke('load_settings');
                settings.selected_preset = state.selectedPreset;
                await window.__TAURI__.core.invoke('save_settings', { settings });
            }
        } catch (error) {
            this.uiManager.logAction('ERROR', `Failed to save selected preset: ${error.message}`);
        }
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'PresetManager');
    }
}