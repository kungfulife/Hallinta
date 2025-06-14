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
        uiManager.showError('Log modal not found');
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
            uiManager.logAction('ERROR', 'Log content element not found');
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
        uiManager.logAction('ERROR', `Error refreshing logs: ${error}`);
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
            uiManager.logAction('INFO', 'Logs cleared');
        }
    } catch (error) {
        uiManager.logAction('ERROR', `Error clearing logs: ${error}`);
    }
};

window.saveLogs = async () => {
    try {
        await window.__TAURI__.core.invoke('flush_log_buffer');
        uiManager.logAction('INFO', 'Logs flushed to daily log file');
    } catch (error) {
        uiManager.logAction('ERROR', `Error flushing logs: ${error}`);
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

    // Show custom menu on .mod-item
    const modList = document.getElementById('mod-list');
    if (modList) {
        modList.addEventListener('contextmenu', (event) => {
            const item = event.target.closest('.mod-item');
            if (!item) return;

            event.preventDefault();
            state.contextMenuTarget = Number(item.dataset.index);

            const mod = state.currentMods[state.contextMenuTarget];
            const menu = document.getElementById('mod-context-menu');
            if (menu) {
                // Toggle visibility of workshop-related buttons based on workshopId
                const copyWorkshopLinkItem = menu.querySelector('#copy-workshop-link');
                const openWorkshopLinkItem = menu.querySelector('#open-workshop-link');
                const isWorkshopMod = mod.workshopId !== '0';
                if (copyWorkshopLinkItem) {
                    copyWorkshopLinkItem.style.display = isWorkshopMod ? 'block' : 'none';
                }
                if (openWorkshopLinkItem) {
                    openWorkshopLinkItem.style.display = isWorkshopMod ? 'block' : 'none';
                }

                let top = event.clientY;
                let left = event.clientX;

                // Ensure menu stays within window bounds
                const menuHeight = menu.offsetHeight || 150; // Fallback if not yet rendered
                const menuWidth = menu.offsetWidth || 200;
                const windowHeight = window.innerHeight;
                const windowWidth = window.innerWidth;

                if (top + menuHeight > windowHeight) {
                    top = windowHeight - menuHeight - 10; // 10px margin
                }
                if (left + menuWidth > windowWidth) {
                    left = windowWidth - menuWidth - 10;
                }
                if (top < 0) top = 10; // Prevent negative top
                if (left < 0) left = 10; // Prevent negative left

                menu.style.top = `${top}px`;
                menu.style.left = `${left}px`;
                menu.style.display = 'block';
            }
        });
    }

    // Hide menu on any click
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
                setTimeout(() => {
                    modManager.finishReordering();
                }, 100);
            },
            onMove: () => true,
        });
    }

    // Debounced log flushing
    setInterval(async () => {
        try {
            await window.__TAURI__.core.invoke('flush_log_buffer');
        } catch (error) {
            uiManager.logAction('ERROR', `Failed to flush log buffer: ${error}`);
        }
    }, 5000); // Every 5 seconds
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