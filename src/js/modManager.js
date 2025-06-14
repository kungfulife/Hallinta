import { state } from './state.js';

export class ModManager {
    constructor(uiManager) {
        this.uiManager = uiManager;
    }

    async loadModConfigFromDirectory(directory) {
        const statusBar = document.getElementById('status-bar');
        try {
            if (window.__TAURI__ && window.__TAURI__.core) {
                const xmlContent = await window.__TAURI__.core.invoke('read_mod_config', { directory });
                this.parseModConfig(xmlContent, true);
                statusBar.textContent = `Loaded ${state.currentMods.length} mods from mod_config.xml`;
                this.uiManager.updateModCount();

                // Start file watching
                await this.startFileWatching(directory);

                // Log the action
                this.logAction('INFO', `Loaded ${state.currentMods.length} mods from ${directory}`);
            }
        } catch (error) {
            console.error('Error loading mod config:', error);
            this.uiManager.showError(`Failed to load mod_config.xml: ${error.message}`);
            this.logAction('ERROR', `Failed to load mod config: ${error.message}`);
        }
    }

    async startFileWatching(directory) {
        if (state.fileWatcher) {
            clearInterval(state.fileWatcher);
        }

        const configPath = `${directory}/mod_config.xml`;

        try {
            // Get initial modification time
            state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', { filePath: configPath });

            // Start watching
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
                    console.error('Error checking file modification:', error);
                }
            }, 2000); // Check every 2 seconds

        } catch (error) {
            console.error('Error starting file watcher:', error);
            this.logAction('ERROR', `Failed to start file watcher: ${error.message}`);
        }
    }

    async handleExternalFileChange(directory) {
        if (state.isReordering) return; // Don't notify during reordering

        const confirmed = confirm('mod_config.xml has been modified externally. Do you want to reload the changes? This will overwrite your current modifications.');

        if (confirmed) {
            await this.loadModConfigFromDirectory(directory);
            this.logAction('INFO', 'Reloaded mod config due to external changes');
        } else {
            // Update the timestamp to ignore this change
            try {
                const configPath = `${directory}/mod_config.xml`;
                state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', { filePath: configPath });
            } catch (error) {
                console.error('Error updating last modified time:', error);
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
            console.error('Error parsing XML:', error);
            document.getElementById('status-bar').textContent = `Error parsing XML: ${error.message}`;
            this.logAction('ERROR', `Failed to parse XML: ${error.message}`);
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

            // Update current preset
            state.currentPresets[state.selectedPreset] = [...state.currentMods];

            // Update last modified time only if file exists
            try {
                const configPath = `${noitaDir}/mod_config.xml`;
                state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', { filePath: configPath });
            } catch (error) {
                console.warn('Could not update last modified time:', error);
                // Don't throw error, just log warning
            }

            this.logAction('INFO', 'Saved mod configuration to file');

        } catch (error) {
            console.error('Error saving mod config:', error);
            document.getElementById('status-bar').textContent = `Error saving mod_config.xml: ${error.message}`;
            this.logAction('ERROR', `Failed to save mod config: ${error.message}`);
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

        const statusBar = document.getElementById('status-bar');
        statusBar.className = 'status-bar';
        statusBar.textContent = `${state.currentMods[index].name} ${state.currentMods[index].enabled ? 'enabled' : 'disabled'}`;

        this.logAction('INFO', `Toggled mod ${state.currentMods[index].name} to ${state.currentMods[index].enabled ? 'enabled' : 'disabled'}`);
    }

    deleteMod(index) {
        const modName = state.currentMods[index].name;
        state.currentMods.splice(index, 1);

        // Update indices
        state.currentMods.forEach((mod, i) => {
            mod.index = i;
        });

        this.uiManager.renderModList();
        this.uiManager.updateModCount();
        this.saveModConfigToFile();

        this.logAction('INFO', `Deleted mod ${modName}`);

        return modName;
    }

    reorderMod(oldIndex, newIndex) {
        state.isReordering = true;

        // Move the mod in the array
        const movedMod = state.currentMods.splice(oldIndex, 1)[0];
        state.currentMods.splice(newIndex, 0, movedMod);

        // Update indices
        state.currentMods.forEach((mod, i) => {
            mod.index = i;
        });

        // Update the UI immediately (fake refresh)
        this.uiManager.renderModList();
        this.uiManager.updateModCount();

        // Mark that we need to save
        state.pendingReorder = true;

        this.logAction('INFO', `Reordered mod from position ${oldIndex + 1} to ${newIndex + 1}`);
    }

    async finishReordering() {
        if (state.pendingReorder) {
            state.isReordering = false;
            state.pendingReorder = false;
            await this.saveModConfigToFile();
            this.logAction('INFO', 'Finished reordering, saved to disk');
        }
    }

    logAction(level, message) {
        if (window.__TAURI__ && window.__TAURI__.core) {
            window.__TAURI__.core.invoke('add_log_entry', {
                level,
                message,
                module: 'ModManager'
            }).catch(console.error);
        }
    }

    // Placeholder for unimplemented functions
    async exportModList() {
        console.log('Export mod list - TBD');
        this.logAction('INFO', 'Export mod list requested');
    }

    async restoreBackup() {
        console.log('Restore backup - TBD');
        this.logAction('INFO', 'Restore backup requested');
    }

    async createBackup() {
        console.log('Create backup - TBD');
        this.logAction('INFO', 'Create backup requested');
    }

    async backupMonitor() {
        console.log('Backup monitor - TBD');
        this.logAction('INFO', 'Backup monitor requested');
    }

    async importRegular() {
        console.log('Import regular - TBD');
        this.logAction('INFO', 'Import regular requested');
    }
}
