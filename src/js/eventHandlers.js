import { state } from './state.js';

export function setupEventHandlers(uiManager, modManager, presetManager, settingsManager) {
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

    window.cancelSettings = () => uiManager.changeView('main');

    window.toggleSettingsView = () => {
        const button = document.getElementById('header-combined-button');
        if (!button) return;

        const isInSettings = button.textContent === 'Cancel';

        if (isInSettings) {
            settingsManager.restorePreviousSettings();
            uiManager.changeView('main');
        } else {
            settingsManager.storeCurrentSettings();
            uiManager.changeView('settings');
        }
    };

    document.addEventListener('DOMContentLoaded', async () => {
        const isDev = window.__TAURI__ ? await window.__TAURI__.core.invoke('is_dev_build') : true;

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
                event.preventDefault();
            }
        });

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

                    const menuHeight = menu.offsetHeight || 150;
                    const menuWidth = menu.offsetWidth || 200;
                    const windowHeight = window.innerHeight;
                    const windowWidth = window.innerWidth;

                    if (top + menuHeight > windowHeight) {
                        top = windowHeight - menuHeight - 10;
                    }
                    if (left + menuWidth > windowWidth) {
                        left = windowWidth - menuWidth - 10;
                    }
                    if (top < 0) top = 10;
                    if (left < 0) left = 10;

                    menu.style.top = `${top}px`;
                    menu.style.left = `${left}px`;
                    menu.style.display = 'block';
                }
            });
        }

        document.addEventListener('click', () => {
            const menu = document.getElementById('mod-context-menu');
            if (menu) {
                menu.style.display = 'none';
            }
        });

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

        await settingsManager.loadConfig();
        presetManager.loadPresets();

        setTimeout(() => {
            if (state.phraseManager) {
                state.phraseManager.startRandomPhrases();
            }
        }, 2000);

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

        setInterval(async () => {
            try {
                await window.__TAURI__.core.invoke('flush_log_buffer');
            } catch (error) {
                uiManager.logAction('ERROR', `Failed to flush log buffer: ${error}`);
            }
        }, 5000);

        window.addEventListener('focus', () => {
            state.isAppFocused = true;
        });

        window.addEventListener('blur', () => {
            state.isAppFocused = false;
        });
    });
}