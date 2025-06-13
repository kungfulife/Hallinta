import { state } from './js/state.js';
import { ModManager } from './js/modManager.js';
import { PresetManager } from './js/presetManager.js';
import { SettingsManager } from './js/settingsManager.js';
import { UIManager } from './js/uiManager.js';
import { PhraseManager } from './js/phraseManager.js';

const uiManager = new UIManager(null);
const modManager = new ModManager(uiManager);
uiManager.modManager = modManager; // Resolve circular dependency
const presetManager = new PresetManager(uiManager);
const settingsManager = new SettingsManager(modManager, uiManager);

state.phraseManager = new PhraseManager();

// Global function bindings
window.changeDirectory = (type) => settingsManager.changeDirectory(type);
window.openDirectory = (type) => settingsManager.openDirectory(type);
window.openAppSettingsFolder = () => settingsManager.openAppSettingsFolder();
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
window.toggleMod = () => uiManager.toggleMod();
window.reorderMod = () => uiManager.reorderMod();
window.deleteMod = () => uiManager.deleteMod();
window.openWorkshop = () => uiManager.openWorkshop();
window.copyWorkshopLink = () => uiManager.copyWorkshopLink();

// Log window functions
window.openLogs = () => {
    const modal = document.getElementById('log-modal');
    if (modal) {
        modal.style.display = 'flex';
        refreshLogs();
    } else {
        console.error('Log modal not found');
    }
};

window.closeLogs = () => {
    const modal = document.getElementById('log-modal');
    if (modal) {
        modal.style.display = 'none';
    }
};

window.refreshLogs = async () => {
    try {
        const logs = await window.__TAURI__.core.invoke('get_log_entries');
        const logContent = document.getElementById('log-content');

        if (!logContent) {
            console.error('Log content element not found');
            return;
        }

        if (logs.length === 0) {
            logContent.textContent = 'No logs available.';
            return;
        }

        const logText = logs.map(log =>
            `[${log.timestamp}] [${log.level}] [${log.module}] ${log.message}`
        ).join('\n');

        logContent.textContent = logText;
        logContent.scrollTop = logContent.scrollHeight;
    } catch (error) {
        console.error('Error refreshing logs:', error);
        const logContent = document.getElementById('log-content');
        if (logContent) {
            logContent.textContent = 'Error loading logs.';
        }
    }
};

window.clearLogs = async () => {
    try {
        await window.__TAURI__.core.invoke('clear_log_buffer');
        const logContent = document.getElementById('log-content');
        if (logContent) {
            logContent.textContent = 'Logs cleared.';
        }
    } catch (error) {
        console.error('Error clearing logs:', error);
    }
};

window.saveLogs = async () => {
    try {
        await window.__TAURI__.core.invoke('save_logs_to_file');
        const statusBar = document.getElementById('status-bar');
        if (statusBar) {
            statusBar.textContent = 'Logs saved to file';
        }
    } catch (error) {
        console.error('Error saving logs:', error);
        const statusBar = document.getElementById('status-bar');
        if (statusBar) {
            statusBar.textContent = 'Error saving logs';
        }
    }
};

document.addEventListener('DOMContentLoaded', async () => {
    const isDev = window.__TAURI__ ? await window.__TAURI__.core.invoke('is_dev_build') : true;

    // Light Dev Tool Restriction
    document.addEventListener('keydown', (e) => {
        if (isDev) return;
        if ((e.ctrlKey && e.key === 'r') || e.key === 'F5') {
            e.preventDefault();
        }
    });

    document.addEventListener('contextmenu', (event) => {
        if (isDev) return;
        const isOnModItem = !!event.target.closest('.mod-item');
        if (!isOnModItem) {
            event.preventDefault(); // block all other right-clicks
        }
    });

    // Show your custom menu on .mod-item
    const modList = document.getElementById('mod-list');
    if (modList) {
        modList.addEventListener('contextmenu', (event) => {
            const item = event.target.closest('.mod-item');
            if (!item) return;

            event.preventDefault();
            state.contextMenuTarget = Number(item.dataset.index);

            const menu = document.getElementById('mod-context-menu');
            if (menu) {
                menu.style.top = `${event.clientY}px`;
                menu.style.left = `${event.clientX}px`;
                menu.style.display = 'block';
            }
        });
    }

    // Hide it on any click
    document.addEventListener('click', () => {
        const menu = document.getElementById('mod-context-menu');
        if (menu) {
            menu.style.display = 'none';
        }
    });

    // Hard-coded keybinds
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            const modal = document.querySelector('.custom-modal');
            const logModal = document.getElementById('log-modal');
            const settingsPage = document.getElementById('settings-page');

            if (modal) {
                modal.remove();
            } else if (logModal && logModal.style.display !== 'none') {
                closeLogs();
            } else if (settingsPage && settingsPage.style.display === 'block') {
                uiManager.changeView('main');
            }
        }
    });

    // App Loading
    await settingsManager.loadConfig();
    presetManager.loadPresets();

    setTimeout(() => {
        if (state.phraseManager) {
            state.phraseManager.startRandomPhrases();
        }
    }, 2000);

    // Sortable setup
    const list = document.getElementById('mod-list');
    if (list) {
        new Sortable(list, {
            animation: 150,
            ghostClass: 'sortable-ghost',
            forceFallback: true,
            onStart: () => {
                state.isReordering = true;
            },
            onEnd: (evt) => {
                modManager.reorderMod(evt.oldIndex, evt.newIndex);
                const statusBar = document.getElementById('status-bar');
                if (statusBar) {
                    statusBar.textContent = `Mod reordered: ${evt.oldIndex + 1} → ${evt.newIndex + 1}`;
                }

                // Finish reordering after a short delay to allow UI updates
                setTimeout(() => {
                    modManager.finishReordering();
                }, 100);
            },
            onMove: () => true,
        });
    }

    // Auto-save logs periodically
    setInterval(async () => {
        try {
            const settings = await window.__TAURI__.core.invoke('load_settings');
            if (settings && settings.log_settings && settings.log_settings.auto_save) {
                await window.__TAURI__.core.invoke('save_logs_to_file');
            }
        } catch (error) {
            console.error('Error in auto-save logs:', error);
        }
    }, 300000); // Every 5 minutes
});

// App focus tracking for file watching
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

window.cancelSettings = () => uiManager.changeView('main');

window.toggleSettingsView = () => {
    const button = document.getElementById('header-combined-button');
    if (!button) return;

    const isInSettings = button.textContent === 'Cancel';

    if (isInSettings) {
        uiManager.changeView('main');
    } else {
        uiManager.changeView('settings');
    }
};
