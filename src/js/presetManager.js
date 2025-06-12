import { state } from './state.js';

export class PresetManager {
    constructor(uiManager) {
        this.uiManager = uiManager;
    }

    loadPresets() {
        const selector = document.getElementById('preset-dropdown');
        selector.innerHTML = '';
        const createOption = document.createElement('option');
        createOption.value = '__create_new__';
        createOption.textContent = '+ Create New Preset';
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

    onPresetChange() {
        const selector = document.getElementById('preset-dropdown');
        const statusBar = document.getElementById('status-bar');
        if (selector.value === '__create_new__') {
            const newName = prompt('Enter name for new preset:', `Preset ${Object.keys(state.currentPresets).length + 1}`);
            if (newName && !state.currentPresets[newName]) {
                state.currentPresets[newName] = [...state.currentMods];
                state.selectedPreset = newName;
                this.loadPresets();
                statusBar.textContent = `Created new preset: ${newName}`;
            } else {
                selector.value = state.selectedPreset;
            }
        } else {
            state.selectedPreset = selector.value;
            state.currentMods = [...state.currentPresets[state.selectedPreset]];
            this.uiManager.renderModList();
            this.uiManager.updateModCount();
            statusBar.textContent = `Switched to preset: ${state.selectedPreset}`;
        }
    }

    deleteCurrentPreset() {
        const statusBar = document.getElementById('status-bar');
        if (state.selectedPreset === 'Default') {
            statusBar.textContent = 'Cannot delete the Default preset';
            return;
        }
        if (window.confirm(`Delete preset "${state.selectedPreset}"?`)) {
            delete state.currentPresets[state.selectedPreset];
            state.selectedPreset = 'Default';
            state.currentMods = [...state.currentPresets[state.selectedPreset]];
            this.uiManager.renderModList();
            this.uiManager.updateModCount();
            this.loadPresets();
            statusBar.textContent = `Deleted preset`;
        }
    }

    renameCurrentPreset() {
        const statusBar = document.getElementById('status-bar');
        if (state.selectedPreset === 'Default') {
            statusBar.textContent = 'Cannot rename the Default preset';
            return;
        }
        const newName = prompt('Enter new name:', state.selectedPreset);
        if (newName && newName !== state.selectedPreset && !state.currentPresets[newName]) {
            state.currentPresets[newName] = state.currentPresets[state.selectedPreset];
            delete state.currentPresets[state.selectedPreset];
            state.selectedPreset = newName;
            this.loadPresets();
            statusBar.textContent = `Renamed preset to: ${newName}`;
        }
    }
}