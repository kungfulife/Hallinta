import { state } from './state.js';

export class ModManager {
    constructor(uiManager) {
        this.uiManager = uiManager;
    }

    async loadModConfigFromDirectory(directory) {
        try {
            if (window.__TAURI__ && window.__TAURI__.core) {
                const xmlContent = await window.__TAURI__.core.invoke('read_mod_config', { directory });
                this.parseModConfig(xmlContent, true);
                this.uiManager.logAction('INFO', `Loaded ${state.currentMods.length} mods from mod_config.xml`);
                this.uiManager.updateModCount();
                await this.startFileWatching(directory);
            }
        } catch (error) {
            this.uiManager.logAction('ERROR', `Failed to load mod_config.xml: ${error.message}`);
        }
    }

    async startFileWatching(directory) {
        if (state.fileWatcher) {
            clearInterval(state.fileWatcher);
        }

        const configPath = `${directory}/mod_config.xml`;

        try {
            state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', { filePath: configPath });
            state.fileWatcher = setInterval(async () => {
                if (!state.isAppFocused) return;
                try {
                    const hasChanged = await window.__TAURI__.core.invoke('check_file_modified', {
                        filePath: configPath,
                        lastModified: state.lastModifiedTime
                    });
                    if (hasChanged) {
                        await this.handleExternalFileChange(directory);
                    }
                } catch (error) {
                    this.uiManager.logAction('ERROR', `Error checking file modification: ${error.message}`);
                }
            }, 2000);
        } catch (error) {
            this.uiManager.logAction('ERROR', `Failed to start file watcher: ${error.message}`);
        }
    }

    async handleExternalFileChange(directory) {
        if (state.isReordering) return;

        const confirmed = confirm('mod_config.xml has been modified externally. Do you want to reload the changes? This will overwrite your current modifications.');
        if (confirmed) {
            await this.loadModConfigFromDirectory(directory);
            this.uiManager.logAction('INFO', 'Reloaded mod config due to external changes');
        } else {
            try {
                const configPath = `${directory}/mod_config.xml`;
                state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', { filePath: configPath });
            } catch (error) {
                this.uiManager.logAction('ERROR', `Error updating last modified time: ${error.message}`);
            }
        }
    }

    parseModConfig(xmlContent, isInitialLoad = false) {
        try {
            const parser = new DOMParser();
            const xmlDoc = parser.parseFromString(xmlContent, 'text/xml');
            const parserError = xmlDoc.querySelector('parsererror');
            if (parserError) {
                throw new Error(`XML parsing failed: ${parserError.textContent}`);
            }

            const mods = xmlDoc.querySelectorAll('Mod');
            state.currentMods = Array.from(mods).map((mod, index) => ({
                name: mod.getAttribute('name') || 'Unknown Mod',
                enabled: mod.getAttribute('enabled') === '1',
                workshopId: mod.getAttribute('workshop_item_id') || '0',
                settingsFoldOpen: mod.getAttribute('settings_fold_open') === '1',
                index: index
            }));

            if (isInitialLoad) {
                state.lastKnownModOrder = [...state.currentMods];
                state.currentPresets[state.selectedPreset] = [...state.currentMods];
            }

            this.uiManager.renderModList();
            this.uiManager.updateModCount();
        } catch (error) {
            this.uiManager.logAction('ERROR', `Error parsing XML: ${error.message}`);
        }
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
                state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', { filePath: configPath });
            } catch (error) {
                this.uiManager.logAction('WARN', `Could not update last modified time: ${error.message}`);
            }
        } catch (error) {
            this.uiManager.logAction('ERROR', `Error saving mod_config.xml: ${error.message}`);
        }
    }

    generateModConfigXML() {
        let xml = '<Mods>\n';
        state.currentMods.forEach(mod => {
            xml += `  <Mod enabled="${mod.enabled ? '1' : '0'}" name="${mod.name}" settings_fold_open="${mod.settingsFoldOpen ? '1' : '0'}" workshop_item_id="${mod.workshopId}" />\n`;
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
        state.pendingReorder = true;
        this.uiManager.logAction('INFO', `Reordered mod from position ${oldIndex + 1} to ${newIndex + 1}`);
    }

    // Reorders mod list, then saves to disk.
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

    // Placeholder for unimplemented functions
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