import { state } from './state.js';

export class ModManager {
    constructor(uiManager) {
        this.uiManager = uiManager;
    }

    async loadModConfigFromDirectory(directory) {
        const statusBar = document.getElementById('status-bar');
        try {
            if (window.__TAURI__ && window.__TAURI__.core) {
                const xmlContent = await window.__TAURI__.core.invoke('read_mod_config', {directory: directory});
                this.parseModConfig(xmlContent, true);
                statusBar.textContent = `Loaded ${state.currentMods.length} mods from mod_config.xml`;
                this.uiManager.updateModCount();
            }
        } catch (error) {
            console.error('Error loading mod config:', error);
            statusBar.textContent = `Error loading mod_config.xml: ${error.message}`;
        }
    }

    parseModConfig(xmlContent, isInitialLoad = false) {
        try {
            const parser = new DOMParser();
            const xmlDoc = parser.parseFromString(xmlContent, 'text/xml');
            const parserError = xmlDoc.querySelector('parsererror');
            if (parserError) {
                throw new Error('XML parsing failed: ' + parserError.textContent);
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
        }
    }

    async saveModConfigToFile() {
        try {
            const noitaDir = document.getElementById('noita-dir').value;
            const xmlContent = this.generateModConfigXML();
            await window.__TAURI__.core.invoke('write_mod_config', {
                directory: noitaDir,
                content: xmlContent
            });
            state.currentPresets[state.selectedPreset] = [...state.currentMods];
            console.log('Mod config saved successfully');
        } catch (error) {
            console.error('Error saving mod config:', error);
            document.getElementById('status-bar').textContent = `Error saving mod_config.xml: ${error.message}`;
        }
    }

    generateModConfigXML() {
        let xml = '<Mods>\n';
        state.currentMods.forEach(mod => {
            xml += `  <Mod enabled="${mod.enabled ? '1' : '0'}" name="${mod.name}" settings_fold_open="${mod.settingsFoldOpen ? '1' : '0'}" workshop_item_id="${mod.workshopId}" >\n  </Mod>\n`;
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
    }

    deleteMod(index) {
        const modName = state.currentMods[index].name;
        state.currentMods.splice(index, 1);
        this.uiManager.renderModList();
        this.uiManager.updateModCount();
        this.saveModConfigToFile();
        return modName;
    }

    reorderMod(oldIndex, newIndex) {
        const movedMod = state.currentMods.splice(oldIndex, 1)[0];
        state.currentMods.splice(newIndex, 0, movedMod);
        state.currentMods.forEach((mod, i) => mod.index = i);
        this.uiManager.renderModList();
        this.uiManager.updateModCount();
        this.saveModConfigToFile();
    }

    // Placeholder for unimplemented functions
    async exportModList() { console.log('Export mod list TBD'); }
    async restoreBackup() { console.log('Restore backup TBD'); }
    async createBackup() { console.log('Create backup TBD'); }
    async backupMonitor() { console.log('Backup monitor TBD'); }
    async importRegular() { console.log('Import regular TBD'); }
}