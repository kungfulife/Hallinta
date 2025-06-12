import { state } from './state.js';
import { ModManager } from './modManager.js';
import { PresetManager } from './presetManager.js';
import { SettingsManager } from './settingsManager.js';
import { UIManager } from './uiManager.js';
import { PhraseManager } from './phraseManager.js';

const uiManager = new UIManager(null);
const modManager = new ModManager(uiManager);
uiManager.modManager = modManager; // Resolve circular dependency
const presetManager = new PresetManager(uiManager);
const settingsManager = new SettingsManager(modManager, uiManager);
state.phraseManager = new PhraseManager();

window.changeView = (view) => uiManager.changeView(view);
window.changeDirectory = (type) => settingsManager.changeDirectory(type);
window.openDirectory = (type) => settingsManager.openDirectory(type);
window.resetToDefaults = () => settingsManager.resetToDefaults();
window.saveAndClose = () => settingsManager.saveAndClose();
window.toggleDarkMode = () => uiManager.toggleDarkMode();
window.filterMods = () => uiManager.filterMods();
window.onPresetChange = () => presetManager.onPresetChange();
window.renameCurrentPreset = () => presetManager.renameCurrentPreset();
window.deleteCurrentPreset = () => presetManager.deleteCurrentPreset();
window.importRegular = () => modManager.importRegular();
window.exportModList = () => modManager.exportModList();
window.restoreBackup = () => modManager.restoreBackup();
window.createBackup = () => modManager.createBackup();
window.backupMonitor = () => modManager.backupMonitor();
window.toggleModEnabled = () => uiManager.toggleModEnabled();
window.reorderMod = () => uiManager.reorderMod();
window.deleteMod = () => uiManager.deleteMod();
window.openWorkshop = () => uiManager.openWorkshop();
window.copyWorkshopLink = () => uiManager.copyWorkshopLink();

async function setupFileWatcher(filePath) {
    console.log('File watcher setup for:', filePath);
}

document.addEventListener('DOMContentLoaded', () => {
    settingsManager.loadConfig();
    presetManager.loadPresets();
    setTimeout(() => {
        state.phraseManager.startRandomPhrases();
    }, 2000);
    const list = document.getElementById('mod-list');
    if (list) {
        new Sortable(list, {
            animation: 150,
            ghostClass: 'sortable-ghost',
            forceFallback: true,
            onEnd: (evt) => {
                modManager.reorderMod(evt.oldIndex, evt.newIndex);
                const statusBar = document.getElementById('status-bar');
                if (statusBar) {
                    statusBar.textContent = `Mod reordered: ${evt.oldIndex + 1} → ${evt.newIndex + 1}`;
                }
            },
            onMove: () => true,
        });
    }
    const contextMenu = document.getElementById('context-menu');
    if (list && contextMenu) {
        list.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            const modItem = e.target.closest('.mod-item');
            if (modItem) {
                state.contextMenuTarget = parseInt(modItem.getAttribute('data-index'));
                contextMenu.style.display = 'block';
                contextMenu.style.left = e.pageX + 'px';
                contextMenu.style.top = e.pageY + 'px';
            }
        });
    }
    if (contextMenu) {
        document.addEventListener('click', () => {
            contextMenu.style.display = 'none';
        });
    }
});

window.addEventListener('focus', () => {
    state.isAppFocused = true;
});

window.addEventListener('blur', () => {
    state.isAppFocused = false;
});

window.addEventListener('beforeunload', () => {
    if (state.phraseManager) {
        state.phraseManager.stopRandomPhrases();
    }
});