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
        const statusBar = document.getElementById('status-bar');

        if (selector.value === 'createnew') {
            const newName = prompt('Enter name for new preset:', `Preset ${Object.keys(state.currentPresets).length + 1}`);
            if (newName && !state.currentPresets[newName]) {
                state.currentPresets[newName] = [...state.currentMods];
                state.selectedPreset = newName;
                this.loadPresets();
                await this.saveSelectedPreset();
                statusBar.textContent = `Created new preset: ${newName}`;
                this.logAction('INFO', `Created new preset: ${newName}`);
            } else {
                selector.value = state.selectedPreset;
            }
        } else {
            state.selectedPreset = selector.value;
            state.currentMods = [...state.currentPresets[state.selectedPreset]];
            this.uiManager.renderModList();
            this.uiManager.updateModCount();
            await this.saveSelectedPreset();
            statusBar.textContent = `Switched to preset: ${state.selectedPreset}`;
            this.logAction('INFO', `Switched to preset: ${state.selectedPreset}`);
        }
    }

    async deleteCurrentPreset() {
        const statusBar = document.getElementById('status-bar');

        if (state.selectedPreset === 'Default') {
            statusBar.textContent = 'Cannot delete the Default preset';
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
            statusBar.textContent = `Deleted preset: ${deletedPreset}`;
            this.logAction('INFO', `Deleted preset: ${deletedPreset}`);
        }
    }

    async renameCurrentPreset() {
        const statusBar = document.getElementById('status-bar');

        if (state.selectedPreset === 'Default') {
            statusBar.textContent = 'Cannot rename the Default preset';
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
            statusBar.textContent = `Renamed preset to: ${newName}`;
            this.logAction('INFO', `Renamed preset from ${oldName} to ${newName}`);
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
            console.error('Error saving selected preset:', error);
            this.logAction('ERROR', `Failed to save selected preset: ${error.message}`);
        }
    }

    logAction(level, message) {
        if (window.__TAURI__ && window.__TAURI__.core) {
            window.__TAURI__.core.invoke('add_log_entry', {
                level,
                message,
                module: 'PresetManager'
            }).catch(console.error);
        }
    }
}
