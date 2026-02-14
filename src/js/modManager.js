import {buildPresetsForSave, state} from './state.js';
export class ModManager {
    constructor(uiManager) {
        this.uiManager = uiManager;
    }

    async loadModConfigFromDirectory(directory) {
        try {
            if (window.__TAURI__ && window.__TAURI__.core) {
                this.logAction('DEBUG', `Loading mod_config.xml from: ${directory}`);
                const xmlContent = await window.__TAURI__.core.invoke('read_mod_config', {directory});
                const wasConsistent = await this.checkPresetConsistency(directory, xmlContent);

                if (wasConsistent) {
                    this.parseModConfig(xmlContent, true);
                    this.logAction('INFO', `Loaded ${state.currentMods.length} mods from mod_config.xml`);
                }

                this.uiManager.updateModCount();

                // Update lastModifiedTime after loading
                try {
                    const configPath = `${directory}/mod_config.xml`;
                    state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', {filePath: configPath});
                } catch (timeError) {
                    this.logAction('WARN', `Could not update last modified time after load: ${timeError.message}`);
                }
            }

            this.logAction('DEBUG', 'Loaded Mod Config from directory');
        } catch (error) {
            let errorMessage = `Failed to load mod_config.xml: ${error.message}`;
            if (error.message.includes("mod_config.xml not found")) {
                errorMessage = "mod_config.xml not found in the specified directory. Please ensure the Noita save directory is correct.";
            }
            this.logAction('ERROR', errorMessage);
        }
    }

    async checkPresetConsistency(directory, xmlContent) {
        const currentPresetMods = state.currentPresets[state.selectedPreset] ||
            [];
        const fileMods = this.parseModsFromXML(xmlContent);

        if (currentPresetMods.length === 0 && fileMods.length > 0) {
            this.logAction('INFO', `Populating empty preset '${state.selectedPreset}' from mod_config.xml.`);
            state.currentPresets[state.selectedPreset] = [...fileMods];

            const presetsForSave = buildPresetsForSave(state.currentPresets);
            await window.__TAURI__.core.invoke('save_presets', {presets: presetsForSave});
            return true;
        }

        const isDifferent = !this.areModsEqual(currentPresetMods, fileMods);
        if (isDifferent) {
            return new Promise((resolve) => {
                this.uiManager.showConfirmModal(
                    `mod_config.xml was modified externally and no longer matches your "${state.selectedPreset}" preset.`, {
                        confirmText: 'Accept External Changes',
                        cancelText: 'Keep Current Preset',
                        onConfirm: async () => {
                            this.logAction('INFO', `Loading changes from mod_config.xml into preset '${state.selectedPreset}'.`);

                            state.currentMods = [...fileMods];
                            state.currentPresets[state.selectedPreset] = [...fileMods];
                            this.uiManager.renderModList();
                            this.uiManager.updateModCount();

                            const presetsForSave = buildPresetsForSave(state.currentPresets);
                            await window.__TAURI__.core.invoke('save_presets', {presets: presetsForSave});
                            resolve(false);
                        },
                        onCancel: async () => {
                            this.logAction('INFO', `Keeping preset '${state.selectedPreset}' and restoring mod_config.xml.`);
                            state.currentMods = state.currentPresets[state.selectedPreset].map(mod => ({...mod}));
                            this.uiManager.renderModList();
                            this.uiManager.updateModCount();
                            await this.saveModConfigToFile();
                            await this.saveSelectedPreset();
                            resolve(false);
                        },
                        isImportant: true
                    }
                );
            });
        }

        return true;
    }

    // TODO : Semi Duplicate code that should not be.
    async saveSelectedPreset() {
        try {
            if (window.__TAURI__ && window.__TAURI__.core) {
                const presetsForSave = buildPresetsForSave(state.currentPresets);
                await window.__TAURI__.core.invoke('save_presets', { presets: presetsForSave });
                this.logAction('INFO', `Saved preset configuration for: ${state.selectedPreset}`);
            }
        } catch (error) {
            this.logAction('ERROR', `Failed to save selected preset: ${error.message}`);
            throw error;
        }
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
            this.logAction('ERROR', `Error parsing XML: ${error.message}`);
            return [];
        }
    }

    parseModConfig(xmlContent, syncToPreset = false) {
        const mods = this.parseModsFromXML(xmlContent);
        state.currentMods = mods;
        if (syncToPreset) {
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

            this.logAction('DEBUG', `Saving mod_config.xml to: ${noitaDir}`);
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
                this.logAction('WARN', `Could not update last modified time: ${error.message}`);
            }
            this.logAction('INFO', 'Saved mod_config.xml');
        } catch (error) {
            this.logAction('ERROR', `Error saving mod_config.xml: ${error.message}`);
        }
    }

    generateModConfigXML() {
        let xml = '<?xml version="1.0" encoding="UTF-8"?>\n<Mods>\n';
        state.currentMods.forEach(mod => {
            xml += `  <Mod name="${mod.name}" enabled="${mod.enabled ? '1' : '0'}" workshop_item_id="${mod.workshopId}" settings_fold_open="${mod.settingsFoldOpen ? '1' : '0'}"></Mod>\n`;
        });
        xml += '</Mods>';
        return xml;
    }

    toggleMod(index) {
        state.currentMods[index].enabled = !state.currentMods[index].enabled;
        this.uiManager.updateModCount();
        this.saveModConfigToFile();
        this.saveSelectedPreset();
        this.logAction('DEBUG', `${state.currentMods[index].name} ${state.currentMods[index].enabled ? 'enabled' : 'disabled'}`);
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
        this.saveSelectedPreset();
        this.logAction('DEBUG', `Deleted mod ${modName}`);
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
        state.pendingReorder = true;
        this.logAction('DEBUG', `Reordered "${movedMod.name}" from position ${oldIndex + 1} to ${newIndex + 1}`);
    }

    async finishReordering() {
        state.isReordering = false;
        if (state.pendingReorder) {
            state.pendingReorder = false;
            await this.saveModConfigToFile();
            await this.saveSelectedPreset();
        }
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'ModManager');
    }

    async exportModList() {
        this.logAction('DEBUG', 'Exporting enabled mods...');
        try {
            const enabledMods = state.currentMods
                .filter(mod => mod.enabled)
                .map(mod => ({
                    name: mod.name,
                    workshopId: mod.workshopId
                }));

            if (enabledMods.length === 0) {
                this.uiManager.logAction('WARN', 'No enabled mods to export.');
                return;
            }

            const filePath = await window.__TAURI__.dialog.save({
                title: 'Export Enabled Mods',
                defaultPath: `${state.selectedPreset}-mod-list.json`,
                filters: [{name: 'JSON', extensions: ['json']}]
            });

            if (filePath) {
                const content = JSON.stringify(enabledMods, null, 2);
                await window.__TAURI__.core.invoke('write_file', {path: filePath, content});
                this.uiManager.logAction('INFO', `Successfully exported ${enabledMods.length} mods.`);
            } else {
                this.uiManager.logAction('INFO', 'Mod export cancelled.');
            }

        } catch (error) {
            this.uiManager.logAction('ERROR', `Failed to export mod list: ${error.message}`);
        }
    }

    async importRegular() {
        this.logAction('DEBUG', 'Import mod list requested');
        try {
            const selectedPath = await window.__TAURI__.dialog.open({
                title: 'Import Mod List',
                multiple: false,
                filters: [{name: 'JSON', extensions: ['json']}]
            });

            if (!selectedPath) {
                this.uiManager.logAction('INFO', 'Mod import cancelled.');
                return;
            }

            const path = Array.isArray(selectedPath) ? selectedPath[0] : selectedPath;

            const content = await window.__TAURI__.core.invoke('read_file', {path: path});
            const importedMods = JSON.parse(content);

            if (!Array.isArray(importedMods)) {
                throw new Error("Import file is not a valid mod list.");
            }

            const allUserMods = state.currentMods;
            const missingMods = [];
            const foundModsInOrder = [];

            const userModsLookup = new Map();
            allUserMods.forEach(m => {
                const key = m.workshopId && m.workshopId !== '0' ? m.workshopId : m.name;
                userModsLookup.set(key, m);
            });

            for (const importedMod of importedMods) {
                const key = importedMod.workshopId && importedMod.workshopId !== '0' ? importedMod.workshopId : importedMod.name;
                if (userModsLookup.has(key)) {
                    foundModsInOrder.push(userModsLookup.get(key));
                } else {
                    missingMods.push(importedMod);
                }
            }

            const proceedWithImport = () => {
                if (foundModsInOrder.length === 0 && missingMods.length > 0) {
                    this.uiManager.logAction('WARN', 'No mods from the import list were found in your current mod list. Aborting.');
                    return;
                }

                const foundModsSet = new Set(foundModsInOrder);
                const otherMods = allUserMods.filter(m => !foundModsSet.has(m));

                foundModsInOrder.forEach(m => m.enabled = true);
                otherMods.forEach(m => m.enabled = false);

                state.currentMods = [...foundModsInOrder, ...otherMods];

                this.logAction('INFO', `Imported and reordered ${foundModsInOrder.length} mods. ${otherMods.length} other mods disabled.`);
                this.uiManager.renderModList();
                this.saveModConfigToFile();
                this.saveSelectedPreset();
            };

            if (missingMods.length > 0) {
                this.uiManager.showMissingModsModal(missingMods, () => {
                    this.logAction('INFO', 'User chose to continue import despite missing mods.');
                    proceedWithImport();
                });
            } else {
                proceedWithImport();
            }

        } catch (error) {
            this.uiManager.logAction('ERROR', `Failed to import mod list: ${error.message}`);
        }
    }
}
