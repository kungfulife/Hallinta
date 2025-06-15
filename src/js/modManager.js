import {state} from './state.js';

export class ModManager {
    constructor(uiManager) {
        this.uiManager = uiManager;
    }

    async loadModConfigFromDirectory(directory) {
        try {
            if (window.__TAURI__ && window.__TAURI__.core) {
                const xmlContent = await window.__TAURI__.core.invoke('read_mod_config', {directory});

                // This will handle user interaction if there's an inconsistency.
                // It returns true if the file was consistent with the preset.
                const wasConsistent = await this.checkPresetConsistency(directory, xmlContent);

                if (wasConsistent) {
                    // If it was consistent, we still need to parse and load the mods into the UI.
                    this.parseModConfig(xmlContent, true);
                }
                // If it was not consistent, the handlers within checkPresetConsistency already updated the UI.

                this.uiManager.logAction('INFO', `Loaded ${state.currentMods.length} mods from mod_config.xml`);
                this.uiManager.updateModCount();
            }
        } catch (error) {
            let errorMessage = `Failed to load mod_config.xml: ${error.message}`;
            if (error.message.includes("mod_config.xml not found")) {
                errorMessage = "mod_config.xml not found in the specified directory. Please ensure the Noita save directory is correct.";
            }
            this.uiManager.showError(errorMessage);
            this.uiManager.logAction('ERROR', errorMessage);
        }
    }

    async checkPresetConsistency(directory, xmlContent) {
        const currentPresetMods = state.currentPresets[state.selectedPreset] || [];
        const fileMods = this.parseModsFromXML(xmlContent);

        if (currentPresetMods.length === 0 && fileMods.length > 0) {
            this.uiManager.logAction('INFO', `Populating empty preset '${state.selectedPreset}' from mod_config.xml.`);
            state.currentPresets[state.selectedPreset] = [...fileMods];

            const presetsForSave = {};
            Object.keys(state.currentPresets).forEach(presetName => {
                presetsForSave[presetName] = state.currentPresets[presetName].map(mod => ({
                    name: mod.name,
                    enabled: mod.enabled,
                    workshop_id: mod.workshopId || '0',
                    settings_fold_open: mod.settingsFoldOpen || false
                }));
            });
            await window.__TAURI__.core.invoke('save_presets', {presets: presetsForSave});
            return true;
        }

        const isDifferent = !this.areModsEqual(currentPresetMods, fileMods);

        if (isDifferent) {
            return new Promise((resolve) => {
                this.uiManager.showConfirmModal(
                    `The mod_config.xml file in your Noita folder has changed and doesn't match the "${state.selectedPreset}" preset. How would you like to proceed?`, {
                        confirmText: 'Load from File',
                        cancelText: 'Overwrite File',
                        onConfirm: async () => {
                            this.uiManager.logAction('INFO', `Loading changes from mod_config.xml into preset '${state.selectedPreset}'.`);
                            // Update both the preset and the current mod list/UI
                            state.currentMods = [...fileMods];
                            state.currentPresets[state.selectedPreset] = [...fileMods];
                            this.uiManager.renderModList();
                            this.uiManager.updateModCount();

                            // Save all presets to presets.json
                            const presetsForSave = {};
                            Object.keys(state.currentPresets).forEach(presetName => {
                                presetsForSave[presetName] = state.currentPresets[presetName].map(mod => ({
                                    name: mod.name,
                                    enabled: mod.enabled,
                                    workshop_id: mod.workshopId || '0',
                                    settings_fold_open: mod.settingsFoldOpen || false
                                }));
                            });
                            await window.__TAURI__.core.invoke('save_presets', {presets: presetsForSave});
                            resolve(false); // Inconsistent
                        },
                        onCancel: async () => {
                            this.uiManager.logAction('INFO', `Overwriting mod_config.xml with preset '${state.selectedPreset}'.`);
                            await this.saveModConfigToFile();
                            resolve(false); // Inconsistent
                        }
                    }
                );
            });
        }

        return true; // Consistent
    }

    areModsEqual(mods1, mods2) {
        if (mods1.length !== mods2.length) return false;
        return mods1.every((mod, i) => (
            mod.name === mods2[i].name &&
            mod.enabled === mods2[i].enabled &&
            mod.workshopId === mods2[i].workshopId &&
            mod.settingsFoldOpen === mods2[i].settingsFoldOpen
        ));
    }

    parseModsFromXML(xmlContent) {
        try {
            const parser = new DOMParser();
            const xmlDoc = parser.parseFromString(xmlContent, 'text/xml');
            const parserError = xmlDoc.querySelector('parsererror');
            if (parserError) {
                throw new Error(`XML parsing failed: ${parserError.textContent}`);
            }
            const mods = xmlDoc.querySelectorAll('Mod');
            return Array.from(mods).map((mod, index) => ({
                name: mod.getAttribute('name') || 'Unknown Mod',
                enabled: mod.getAttribute('enabled') === '1',
                workshopId: mod.getAttribute('workshop_item_id') || '0',
                settingsFoldOpen: mod.getAttribute('settings_fold_open') === '1',
                index: index
            }));
        } catch (error) {
            this.uiManager.logAction('ERROR', `Error parsing XML: ${error.message}`);
            return []; // Return empty array on error
        }
    }

    parseModConfig(xmlContent, isInitialLoad = false) {
        const mods = this.parseModsFromXML(xmlContent);
        state.currentMods = mods;
        if (isInitialLoad) {
            state.lastKnownModOrder = [...state.currentMods];
            state.currentPresets[state.selectedPreset] = [...state.currentMods];
        }
        this.uiManager.renderModList();
        this.uiManager.updateModCount();
    }

    async saveModConfigToFile() {
        try {
            const noitaDirElement = document.getElementById('noita-dir');
            const noitaDir = noitaDirElement ? noitaDirElement.value : '';
            if (!noitaDir) {
                throw new Error('Noita directory not set');
            }

            const xmlContent = this.generateModConfigXML();
            await window.__TAURI__.core.invoke('write_mod_config', {
                directory: noitaDir,
                content: xmlContent
            });
            state.currentPresets[state.selectedPreset] = [...state.currentMods];
            try {
                const configPath = `${noitaDir}/mod_config.xml`;
                state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', {filePath: configPath});
            } catch (error) {
                this.uiManager.logAction('WARN', `Could not update last modified time: ${error.message}`);
            }
            this.uiManager.logAction('INFO', 'Saved mod_config.xml');
        } catch (error) {
            this.uiManager.showError(`Error saving mod_config.xml: ${error.message}`);
            this.uiManager.logAction('ERROR', `Error saving mod_config.xml: ${error.message}`);
        }
    }

    generateModConfigXML() {
        let xml = '<Mods>\n';
        state.currentMods.forEach(mod => {
            xml += `  <Mod name="${mod.name}" enabled="${mod.enabled ? '1' : '0'}" workshop_item_id="${mod.workshopId}" settings_fold_open="${mod.settingsFoldOpen ? '1' : '0'}">\n  </Mod>\n`;
        });
        xml += '</Mods>';
        return xml;
    }

    toggleMod(index) {
        state.currentMods[index].enabled = !state.currentMods[index].enabled;
        this.uiManager.updateModCount();
        this.saveModConfigToFile();
        this.uiManager.logAction('INFO', `${state.currentMods[index].name} ${state.currentMods[index].enabled ? 'enabled' : 'disabled'}`);
    }

    deleteMod(index) {
        const modName = state.currentMods[index].name;
        state.currentMods.splice(index, 1);
        state.currentMods.forEach((mod, i) => {
            mod.index = i;
        });
        this.uiManager.renderModList();
        this.uiManager.updateModCount();
        this.saveModConfigToFile();
        this.uiManager.logAction('INFO', `Deleted mod ${modName}`);
        return modName;
    }

    reorderMod(oldIndex, newIndex) {
        state.isReordering = true;
        const movedMod = state.currentMods.splice(oldIndex, 1)[0];
        state.currentMods.splice(newIndex, 0, movedMod);
        state.currentMods.forEach((mod, i) => {
            mod.index = i;
        });
        this.uiManager.renderModList();
        this.uiManager.updateModCount();
        this.saveModConfigToFile();
        state.pendingReorder = true;
        this.uiManager.logAction('INFO', `Reordered mod from position ${oldIndex + 1} to ${newIndex + 1}`);
    }

    async finishReordering() {
        if (state.pendingReorder) {
            state.isReordering = false;
            state.pendingReorder = false;
            await this.saveModConfigToFile();
        }
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'ModManager');
    }

    async exportModList() {
        this.logAction('INFO', 'Export mod list requested');
    }

    async restoreBackup() {
        this.logAction('INFO', 'Restore backup requested');
    }

    async createBackup() {
        this.logAction('INFO', 'Create backup requested');
    }

    async backupMonitor() {
        this.logAction('INFO', 'Backup monitor requested');
    }

    async importRegular() {
        this.logAction('INFO', 'Import regular requested');
    }

}