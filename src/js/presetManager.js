import {buildPresetsForSave, deepCopyMods, state} from './state.js';

export class PresetManager {
    constructor(uiManager, modManager, settingsManager) {
        this.uiManager = uiManager;
        this.modManager = modManager;
        this.settingsManager = settingsManager;
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

        this.logAction('INFO', 'Loaded Presets');
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
                this.logAction('DEBUG', 'Creating new preset');
                selector.value = state.selectedPreset;
                this.uiManager.showInputModal(
                    'Enter name for new preset:',
                    `Preset ${Object.keys(state.currentPresets).length + 1}`,
                    async (newName) => {
                        if (newName && newName.trim() !== '' && !state.currentPresets[newName]) {
                            state.currentPresets[newName] = [...state.currentMods];
                            state.selectedPreset = newName;
                            await this.saveSelectedPreset();
                            this.loadPresets();
                            this.uiManager.logAction('INFO', `Created new preset: ${newName}`);
                        } else {
                            this.uiManager.logAction('INFO', 'Invalid preset name or preset already exists');
                        }
                    },
                    () => {
                        this.uiManager.logAction('INFO', 'Preset creation canceled');
                    }
                );
            } else if (state.currentPresets[selectedValue] && Array.isArray(state.currentPresets[selectedValue])) {
                const prevPreset = state.selectedPreset;
                const newModCount = state.currentPresets[selectedValue].length;
                this.logAction('DEBUG', `Switching from preset "${prevPreset}" to "${selectedValue}" (${newModCount} mods)`);
                state.selectedPreset = selectedValue;

                await this.loadToSelectedPreset();

                this.uiManager.logAction('INFO', `Switched to preset: ${state.selectedPreset}`);
            } else {
                this.logAction('ERROR', `Preset ${selectedValue} is invalid or not found`);
                selector.value = state.selectedPreset;
            }
        } catch (error) {
            this.logAction('ERROR', `Error changing preset: ${error.message}`);
            selector.value = state.selectedPreset;
        }
    }

    async loadToSelectedPreset() {
        state.currentMods = deepCopyMods(state.currentPresets[state.selectedPreset]);
        this.uiManager.renderModList();
        this.uiManager.updateModCount();

        await this.modManager.saveModConfigToFile();
        await this.saveSelectedPreset();
    }

    async deleteCurrentPreset() {
        this.logAction('DEBUG', `Attempting to delete preset: ${state.selectedPreset}`);
        if (state.selectedPreset === 'Default') {
            this.logAction('WARN', 'Cannot delete default preset');
            return;
        }

        this.uiManager.showConfirmModal(
            `Are you sure you want to delete the preset "${state.selectedPreset}"?`, {
                confirmText: 'Delete',
                cancelText: 'Cancel',
                onConfirm: async () => {
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
                },
                onCancel: () => {
                    this.logAction('INFO', `Deletion of preset "${state.selectedPreset}" canceled.`);
                }
            }
        );
    }

    async renameCurrentPreset() {
        this.logAction('DEBUG', `Attempting to rename preset: ${state.selectedPreset}`);
        if (state.selectedPreset === 'Default') {
            this.logAction('ERROR', 'Cannot rename default preset');
            return;
        }

        this.uiManager.showInputModal(
            `Enter new name for "${state.selectedPreset}":`,
            state.selectedPreset,
            async (newName) => {
                if (newName && newName.trim() !== '' && newName !== state.selectedPreset && !state.currentPresets[newName]) {
                    const oldName = state.selectedPreset;
                    state.currentPresets[newName] = state.currentPresets[state.selectedPreset];
                    delete state.currentPresets[state.selectedPreset];
                    state.selectedPreset = newName;
                    await this.saveSelectedPreset();
                    this.loadPresets();
                    this.logAction('INFO', `Renamed preset from ${oldName} to ${newName}`);
                } else {
                    this.logAction('ERROR', 'Invalid preset name or preset already exists');
                }
            },
            () => {
                this.logAction('INFO', 'Preset rename canceled');
            },

        );
    }

    // TODO: Duplicate code
    async saveSelectedPreset() {
        try {
            if (window.__TAURI__ && window.__TAURI__.core && this.settingsManager) {
                this.settingsManager.settings.selected_preset = state.selectedPreset || 'Default';

                const presetsForSave = buildPresetsForSave(state.currentPresets);
                const settingsToSave = this.settingsManager.getSettingsForPersistence();
                await window.__TAURI__.core.invoke('save_settings', { settings: settingsToSave });
                await window.__TAURI__.core.invoke('save_presets', { presets: presetsForSave });
                this.logAction('INFO', `Saved preset configuration for: ${state.selectedPreset}`);
                this.logAction('DEBUG', `Persisted selected_preset in settings: ${this.settingsManager.settings.selected_preset}`);
            }
        } catch (error) {
            this.logAction('ERROR', `Failed to save selected preset: ${error.message}`);
            throw error;
        }
    }

    async exportPresets() {
        this.logAction('DEBUG', 'Export presets requested');
        const presetNames = Object.keys(state.currentPresets);
        if (presetNames.length === 0) {
            this.logAction('WARN', 'No presets to export');
            return;
        }

        const items = presetNames.map(name => ({
            id: name,
            label: `${name} (${state.currentPresets[name].length} mods)`,
            checked: true
        }));

        this.uiManager.showChecklistModal(
            'Export Presets',
            'Select presets to export:',
            items,
            async (selected) => {
                if (selected.length === 0) {
                    this.logAction('INFO', 'No presets selected for export');
                    return;
                }

                try {
                    const exportData = {
                        hallinta_export: 'presets',
                        version: '0.4.0',
                        presets: {}
                    };

                    selected.forEach(name => {
                        exportData.presets[name] = state.currentPresets[name].map(mod => ({
                            name: mod.name,
                            enabled: mod.enabled,
                            workshop_id: mod.workshopId || '0',
                            settings_fold_open: mod.settingsFoldOpen || false
                        }));
                    });

                    const filePath = await window.__TAURI__.dialog.save({
                        title: 'Export Presets',
                        defaultPath: 'hallinta-presets.json',
                        filters: [{ name: 'JSON', extensions: ['json'] }]
                    });

                    if (filePath) {
                        const content = JSON.stringify(exportData, null, 2);
                        await window.__TAURI__.core.invoke('write_file', { path: filePath, content });
                        this.logAction('INFO', `Exported ${selected.length} preset(s) to file`);
                    } else {
                        this.logAction('INFO', 'Preset export cancelled');
                    }
                } catch (error) {
                    this.logAction('ERROR', `Failed to export presets: ${error.message}`);
                }
            },
            () => {
                this.logAction('INFO', 'Preset export cancelled');
            }
        );
    }

    async importPresets() {
        this.logAction('DEBUG', 'Import presets requested');
        try {
            const selectedPath = await window.__TAURI__.dialog.open({
                title: 'Import Presets',
                multiple: false,
                filters: [{ name: 'JSON', extensions: ['json'] }]
            });

            if (!selectedPath) {
                this.logAction('INFO', 'Preset import cancelled');
                return;
            }

            const path = Array.isArray(selectedPath) ? selectedPath[0] : selectedPath;
            const content = await window.__TAURI__.core.invoke('read_file', { path });
            const importData = JSON.parse(content);

            if (importData.hallinta_export !== 'presets' || !importData.presets) {
                this.logAction('ERROR', 'Invalid preset file format. Expected a Hallinta preset export.');
                return;
            }

            const importedNames = Object.keys(importData.presets);
            if (importedNames.length === 0) {
                this.logAction('WARN', 'No presets found in file');
                return;
            }

            const items = importedNames.map(name => ({
                id: name,
                label: `${name} (${importData.presets[name].length} mods)${state.currentPresets[name] ? ' - EXISTS' : ''}`,
                checked: true
            }));

            this.uiManager.showChecklistModal(
                'Import Presets',
                'Select presets to import:',
                items,
                async (selected) => {
                    if (selected.length === 0) {
                        this.logAction('INFO', 'No presets selected for import');
                        return;
                    }

                    // Check for conflicts
                    const conflicts = selected.filter(name => state.currentPresets[name]);

                    const doImport = async () => {
                        let imported = 0;
                        for (const name of selected) {
                            let targetName = name;
                            // If conflict exists and wasn't explicitly overwriting, add suffix
                            if (state.currentPresets[name] && conflicts.length > 0 && !this._overwriteConflicts) {
                                targetName = `${name} (imported)`;
                                let counter = 2;
                                while (state.currentPresets[targetName]) {
                                    targetName = `${name} (imported ${counter})`;
                                    counter++;
                                }
                            }

                            state.currentPresets[targetName] = importData.presets[name].map(mod => ({
                                name: mod.name,
                                enabled: mod.enabled,
                                workshopId: mod.workshop_id || '0',
                                settingsFoldOpen: mod.settings_fold_open || false,
                                index: 0
                            }));
                            imported++;
                        }

                        this._overwriteConflicts = false;
                        await this.saveSelectedPreset();
                        this.loadPresets();
                        this.logAction('INFO', `Imported ${imported} preset(s)`);
                    };

                    if (conflicts.length > 0) {
                        this.uiManager.showConfirmModal(
                            `${conflicts.length} preset(s) already exist: ${conflicts.join(', ')}. Overwrite them?`,
                            {
                                confirmText: 'Overwrite',
                                cancelText: 'Rename',
                                onConfirm: async () => {
                                    this._overwriteConflicts = true;
                                    await doImport();
                                },
                                onCancel: async () => {
                                    this._overwriteConflicts = false;
                                    await doImport();
                                }
                            }
                        );
                    } else {
                        await doImport();
                    }
                },
                () => {
                    this.logAction('INFO', 'Preset import cancelled');
                }
            );
        } catch (error) {
            this.logAction('ERROR', `Failed to import presets: ${error.message}`);
        }
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'PresetManager');
    }
}
