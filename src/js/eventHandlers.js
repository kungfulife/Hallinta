import {state} from './state.js';

export function setupEventHandlers(uiManager, modManager, presetManager, settingsManager) {
    window.changeDirectory = (type) => settingsManager.changeDirectory(type);
    window.findDefaultDirectory = (type) => settingsManager.findDefaultDirectory(type);
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
        uiManager.logAction('DEBUG', 'Opening logs modal', 'EventHandler');
        if (state.isModalVisible) {
            uiManager.logAction('INFO', 'Cannot open logs while another modal is active.', 'EventHandler');
            return;
        }

        const modal = document.getElementById('log-modal');
        if (modal) {
            modal.style.display = 'flex';
            refreshLogs();
        } else {
            uiManager.logAction('ERROR', 'Log modal not found', 'EventHandler');
        }
    };

    window.closeLogs = () => {
        const modal = document.getElementById('log-modal');
        if (modal) {
            uiManager.logAction('DEBUG', 'Closing logs modal', 'EventHandler');
            modal.style.display = 'none';
        }
    };

    window.refreshLogs = async () => {
        uiManager.logAction('DEBUG', 'Refreshing logs', 'EventHandler');
        try {
            const logs = await window.__TAURI__.core.invoke('get_log_entries');
            const logContent = document.getElementById('log-content');

            if (!logContent) {
                uiManager.logAction('ERROR', 'Log content element not found', 'EventHandler');
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
            uiManager.logAction('ERROR', `Error refreshing logs: ${error}`, 'EventHandler');
            const logContent = document.getElementById('log-content');
            if (logContent) {
                logContent.textContent = 'Error loading logs.';
            }
        }
    };

    window.clearLogs = async () => {
        uiManager.logAction('DEBUG', 'Clearing logs', 'EventHandler');
        try {
            await window.__TAURI__.core.invoke('clear_log_buffer');
            const logContent = document.getElementById('log-content');
            if (logContent) {
                logContent.textContent = 'Logs cleared.';
                uiManager.logAction('INFO', 'Logs cleared', 'EventHandler');
            }
        } catch (error) {
            uiManager.logAction('ERROR', `Error clearing logs: ${error}`, 'EventHandler');
        }
    };

    window.saveLogs = async () => {
        uiManager.logAction('DEBUG', 'Saving logs', 'EventHandler');
        try {
            await window.__TAURI__.core.invoke('flush_log_buffer');
            uiManager.logAction('INFO', 'Logs flushed to log file', 'EventHandler');
        } catch (error) {
            uiManager.logAction('ERROR', `Error flushing logs: ${error}`, 'EventHandler');
        }
    };

    window.cancelSettings = () => uiManager.changeView('main');

    window.toggleSettingsView = () => {
        uiManager.logAction('DEBUG', 'Toggling settings view', 'EventHandler');
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

    let fileCheckInterval = null;
    let fileCheckInProgress = false;

    const stopFileWatcher = () => {
        if (fileCheckInterval) {
            clearInterval(fileCheckInterval);
            fileCheckInterval = null;
        }
    };

    const performFileCheck = async () => {
        if (fileCheckInProgress) return;
        if (state.isModalVisible || state.isReordering || !settingsManager.settings.noita_dir) {
            return;
        }

        fileCheckInProgress = true;
        try {
            const configPath = `${settingsManager.settings.noita_dir}/mod_config.xml`;
            const fileExists = await window.__TAURI__.core.invoke('check_file_exists', {path: configPath});
            if (!fileExists) return;

            const hasChanged = await window.__TAURI__.core.invoke('check_file_modified', {
                filePath: configPath,
                lastModified: state.lastModifiedTime,
            });

            if (hasChanged) {
                stopFileWatcher();
                uiManager.logAction('INFO', 'External change detected for mod_config.xml.', 'EventHandler');

                const xmlContent = await window.__TAURI__.core.invoke('read_mod_config', {directory: settingsManager.settings.noita_dir});
                await modManager.checkPresetConsistency(settingsManager.settings.noita_dir, xmlContent);

                state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', {filePath: configPath});
                startFileWatcher();
            }
        } catch (error) {
            uiManager.logAction('ERROR', `Error during file check: ${error.message}`, 'EventHandler');
            stopFileWatcher();
            setTimeout(startFileWatcher, 5000);
        } finally {
            fileCheckInProgress = false;
        }
    };

    const startFileWatcher = () => {
        stopFileWatcher();
        fileCheckInterval = setInterval(() => {
            if (state.isAppFocused) {
                performFileCheck();
            }
        }, 5000);
    };

    window.addEventListener('focus', () => {
        state.isAppFocused = true;
        performFileCheck();
    });

    window.addEventListener('blur', () => {
        state.isAppFocused = false;
    });

    document.addEventListener('DOMContentLoaded', async () => {

        uiManager.logAction('INFO', 'Setting up event handlers', 'DOMContentLoaded');
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

                uiManager.logAction('DEBUG', `Opening context menu on: ${mod.name}`, 'EventHandler');

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
            if (menu && menu.style.display !== 'none') {
                uiManager.logAction('DEBUG', 'Hiding context menu', 'EventHandler');
                menu.style.display = 'none';
            }
        });

        // TODO: Confirm that there are no memory leaks when closing Logs/Settings via Escape Key.
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                const logModal = document.getElementById('log-modal');
                const settingsPage = document.getElementById('settings-page');

                if (logModal && logModal.style.display !== 'none') {
                    closeLogs();
                } else if (settingsPage && settingsPage.style.display === 'block') {
                    settingsManager.restorePreviousSettings();
                    uiManager.changeView('main');
                }
            }
        });

        await settingsManager.loadConfig();
        presetManager.loadPresets();

        if (settingsManager.settings.noita_dir) {
            const configPath = `${settingsManager.settings.noita_dir}/mod_config.xml`;
            try {
                const fileExists = await window.__TAURI__.core.invoke('check_file_exists', {path: configPath});
                if (fileExists) {
                    state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', {filePath: configPath});
                }
            } catch (e) {
                uiManager.logAction('WARN', `Could not get initial mod time: ${e.message}`, 'DOMContentLoaded');
            }
        }

        startFileWatcher();

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
                    uiManager.logAction('DEBUG', 'Starting mod reorder', 'EventHandler');
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
                uiManager.logAction('ERROR', `Failed to flush log buffer: ${error}`, 'DOMContentLoaded');
            }
        }, 5000);
    });
}