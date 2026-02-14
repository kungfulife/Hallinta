import {deepCopyMods, state} from './state.js';
import {updateDragVisualNumbersByDom} from './reorderUtils.js';

export function setupEventHandlers(uiManager, modManager, presetManager, settingsManager, backupManager) {
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
    window.createBackup = () => backupManager.createBackup();
    window.openRestoreUI = () => backupManager.openRestoreUI();
    window.exportPresets = () => presetManager.exportPresets();
    window.importPresets = () => presetManager.importPresets();
    window.toggleMod = () => uiManager.toggleMod();
    window.reorderMod = () => uiManager.reorderMod();
    window.deleteMod = () => uiManager.deleteMod();
    window.openWorkshop = () => uiManager.openWorkshop();
    window.copyWorkshopLink = () => uiManager.copyWorkshopLink();

    // --- Log Viewer ---

    let logAutoRefreshInterval = null;
    let logViewMode = 'closed'; // 'closed', 'modal', 'fullscreen', 'detached'
    let logDetachedPreference = false; // sticky: once detached, clicking status bar reopens window
    let logLastInAppMode = 'modal'; // tracks what in-app mode was used before detaching
    let logAutoScale = true; // auto-scale modal with window resize
    let logWindowRef = null; // reference to the separate WebviewWindow

    const startLogAutoRefresh = () => {
        stopLogAutoRefresh();
        logAutoRefreshInterval = setInterval(() => {
            refreshLogs();
        }, 1500);
    };

    const stopLogAutoRefresh = () => {
        if (logAutoRefreshInterval) {
            clearInterval(logAutoRefreshInterval);
            logAutoRefreshInterval = null;
        }
    };

    const getActiveLogElements = () => {
        if (logViewMode === 'fullscreen') {
            return {
                content: document.getElementById('log-fs-content'),
                filterLevel: document.getElementById('log-fs-filter-level'),
                search: document.getElementById('log-fs-search'),
            };
        }
        return {
            content: document.getElementById('log-content'),
            filterLevel: document.getElementById('log-filter-level'),
            search: document.getElementById('log-search'),
        };
    };

    const syncFilters = (fromMode) => {
        const srcLevelId = fromMode === 'modal' ? 'log-filter-level' : 'log-fs-filter-level';
        const dstLevelId = fromMode === 'modal' ? 'log-fs-filter-level' : 'log-filter-level';
        const srcLevel = document.getElementById(srcLevelId);
        const dstLevel = document.getElementById(dstLevelId);
        if (srcLevel && dstLevel) dstLevel.value = srcLevel.value;

        const srcSearch = document.getElementById(fromMode === 'modal' ? 'log-search' : 'log-fs-search');
        const dstSearch = document.getElementById(fromMode === 'modal' ? 'log-fs-search' : 'log-search');
        if (srcSearch && dstSearch) dstSearch.value = srcSearch.value;
    };

    const setStatusBarVisible = (visible) => {
        const statusBar = document.getElementById('status-bar');
        if (statusBar) {
            statusBar.style.display = visible ? '' : 'none';
        }
    };

    // Auto-scale modal size to 80% x 70% of window
    const applyAutoScale = () => {
        if (!logAutoScale) return;
        const modalContent = document.querySelector('.log-modal-content');
        if (!modalContent) return;
        modalContent.style.width = '';
        modalContent.style.height = '';
    };

    // Detect user manual resize of the modal
    const setupModalResizeDetection = () => {
        const modalContent = document.querySelector('.log-modal-content');
        if (!modalContent || modalContent._resizeObserverAttached) return;

        let isWindowResize = false;
        window.addEventListener('resize', () => {
            isWindowResize = true;
            if (logAutoScale && logViewMode === 'modal') {
                applyAutoScale();
            } else if (!logAutoScale && logViewMode === 'modal') {
                // Check if user's manual size exceeds viewport
                const rect = modalContent.getBoundingClientRect();
                if (rect.width > window.innerWidth * 0.92 || rect.height > window.innerHeight * 0.82) {
                    logAutoScale = true;
                    applyAutoScale();
                }
            }
            requestAnimationFrame(() => { isWindowResize = false; });
        });

        const observer = new ResizeObserver(() => {
            if (isWindowResize || logViewMode !== 'modal') return;
            // User manually resized the modal
            logAutoScale = false;
        });
        observer.observe(modalContent);
        modalContent._resizeObserverAttached = true;
    };

    window.openLogs = () => {
        uiManager.logAction('DEBUG', 'Opening logs', 'EventHandler');

        // If detached preference is active, reopen separate window
        if (logDetachedPreference) {
            openLogWindow();
            return;
        }

        if (state.isModalVisible) {
            uiManager.logAction('INFO', 'Cannot open logs while another modal is active.', 'EventHandler');
            return;
        }

        const modal = document.getElementById('log-modal');
        if (modal) {
            logViewMode = 'modal';
            logLastInAppMode = 'modal';
            modal.style.display = 'flex';
            initLogFilterDropdown();
            applyAutoScale();
            setupModalResizeDetection();
            refreshLogs();
            startLogAutoRefresh();
        }
    };

    window.closeLogs = () => {
        uiManager.logAction('DEBUG', 'Closing logs', 'EventHandler');
        const modal = document.getElementById('log-modal');
        const fullscreen = document.getElementById('log-fullscreen');
        if (modal) modal.style.display = 'none';
        if (fullscreen) fullscreen.style.display = 'none';

        if (logViewMode === 'modal' || logViewMode === 'fullscreen') {
            logLastInAppMode = logViewMode;
        }
        logViewMode = 'closed';
        stopLogAutoRefresh();
    };

    window.toggleLogFullscreen = () => {
        const modal = document.getElementById('log-modal');
        const fullscreen = document.getElementById('log-fullscreen');

        if (logViewMode === 'modal') {
            syncFilters('modal');
            if (modal) modal.style.display = 'none';
            if (fullscreen) fullscreen.style.display = 'block';
            logViewMode = 'fullscreen';
            logLastInAppMode = 'fullscreen';
            uiManager.logAction('DEBUG', 'Switched to fullscreen log view', 'EventHandler');
        } else if (logViewMode === 'fullscreen') {
            syncFilters('fullscreen');
            if (fullscreen) fullscreen.style.display = 'none';
            if (modal) modal.style.display = 'flex';
            logViewMode = 'modal';
            logLastInAppMode = 'modal';
            applyAutoScale();
            uiManager.logAction('DEBUG', 'Switched to modal log view', 'EventHandler');
        }
        refreshLogs();
    };

    const openLogWindow = async () => {
        uiManager.logAction('DEBUG', 'Opening separate log window', 'EventHandler');

        // If window already exists, try to focus it instead of creating a new one
        if (logWindowRef) {
            try {
                await logWindowRef.setFocus();
                logViewMode = 'detached';
                setStatusBarVisible(false);
                return;
            } catch (e) {
                // Window was destroyed, proceed to create new one
                logWindowRef = null;
            }
        }

        // Close any in-app log views first
        const modal = document.getElementById('log-modal');
        const fullscreen = document.getElementById('log-fullscreen');
        if (modal) modal.style.display = 'none';
        if (fullscreen) fullscreen.style.display = 'none';
        stopLogAutoRefresh();

        logViewMode = 'detached';
        logDetachedPreference = true;
        setStatusBarVisible(false);

        try {
            const WebviewWindow = window.__TAURI__.webviewWindow.WebviewWindow;
            logWindowRef = new WebviewWindow('log-window', {
                url: 'log-window.html',
                title: 'Hallinta - Application Logs',
                width: 900,
                height: 600,
                center: true,
                resizable: true,
                decorations: true,
            });

            logWindowRef.once('tauri://error', (e) => {
                uiManager.logAction('ERROR', `Failed to open log window: ${e}`, 'EventHandler');
                logViewMode = 'closed';
                setStatusBarVisible(true);
            });

            // Listen for window close (user closed via X button)
            logWindowRef.once('tauri://destroyed', () => {
                logWindowRef = null;
                logViewMode = 'closed';
                setStatusBarVisible(true);
                // logDetachedPreference stays true — clicking status bar reopens window
            });
        } catch (error) {
            uiManager.logAction('ERROR', `Error opening log window: ${error}`, 'EventHandler');
            logViewMode = 'closed';
            setStatusBarVisible(true);
        }
    };
    window.openLogWindow = openLogWindow;

    // Called from separate window via Tauri event — return logs to in-app mode
    const returnLogsToApp = async () => {
        logDetachedPreference = false;
        logViewMode = 'closed';

        // Close the separate window if still open
        if (logWindowRef) {
            try {
                await logWindowRef.close();
            } catch (e) {
                // Window may already be closed
            }
            logWindowRef = null;
        }

        setStatusBarVisible(true);

        // Reopen in-app using last mode
        initLogFilterDropdown();
        if (logLastInAppMode === 'fullscreen') {
            const fullscreen = document.getElementById('log-fullscreen');
            if (fullscreen) {
                logViewMode = 'fullscreen';
                fullscreen.style.display = 'block';
                refreshLogs();
                startLogAutoRefresh();
            }
        } else {
            const modalEl = document.getElementById('log-modal');
            if (modalEl) {
                logViewMode = 'modal';
                modalEl.style.display = 'flex';
                applyAutoScale();
                refreshLogs();
                startLogAutoRefresh();
            }
        }
    };

    // Listen for "return-to-app" event from the separate log window
    if (window.__TAURI__?.event) {
        window.__TAURI__.event.listen('log-return-to-app', () => {
            returnLogsToApp();
        });
    }

    window.copyLogs = async () => {
        const els = getActiveLogElements();
        if (!els.content) return;

        const logLines = els.content.querySelectorAll('.log-line');
        const text = Array.from(logLines).map(line => line.textContent).join('\n');

        try {
            await navigator.clipboard.writeText(text);
            uiManager.logAction('INFO', 'Logs copied to clipboard', 'EventHandler');
        } catch (error) {
            uiManager.logAction('ERROR', `Error copying logs: ${error}`, 'EventHandler');
        }
    };

    const refreshLogs = async () => {
        // Don't refresh in-app views when detached to separate window
        if (logViewMode === 'detached') return;

        try {
            const logs = await window.__TAURI__.core.invoke('get_log_entries');
            const els = getActiveLogElements();

            if (!els.content) return;

            const selectedLevel = els.filterLevel?.value || 'INFO';
            const searchText = els.search?.value || '';

            // Smart scroll: check if user is near bottom before updating
            const isNearBottom = els.content.scrollTop + els.content.clientHeight >= els.content.scrollHeight - 30;

            els.content.innerHTML = window.logUtils.buildLogHTML(logs, selectedLevel, searchText);

            if (isNearBottom) {
                els.content.scrollTop = els.content.scrollHeight;
            }
        } catch (error) {
            uiManager.logAction('ERROR', `Error refreshing logs: ${error}`, 'EventHandler');
            const els = getActiveLogElements();
            if (els.content) {
                els.content.innerHTML = '<div class="log-line log-error"><span class="log-msg">Error loading logs.</span></div>';
            }
        }
    };

    const initLogFilterDropdown = () => {
        const currentLogLevel = settingsManager.settings?.log_settings?.log_level || 'INFO';
        window.logUtils.initLogFilterDropdown(['log-filter-level', 'log-fs-filter-level'], currentLogLevel);
    };

    window.refreshLogs = refreshLogs;
    window.openSystemInfo = async () => {
        const panel = document.getElementById('system-info-panel');
        const body = document.getElementById('system-info-body');
        if (!panel || !body) return;

        try {
            const info = await window.__TAURI__.core.invoke('get_system_info');
            const rows = [
                ['App Version', info.app_version],
                ['Build Mode', info.build_mode],
                ['Rust Version', info.rust_version],
                ['Cargo Version', info.cargo_version],
                ['Target Triple', info.target_triple],
                ['Tauri Version', info.tauri_version],
                ['OS', info.os],
                ['Architecture', info.arch],
                ['Data Directory', info.data_dir]
            ];
            body.innerHTML = rows.map(([label, value]) => (
                `<div class="system-info-row"><strong>${window.logUtils.escapeHtml(label)}</strong><span>${window.logUtils.escapeHtml(value || '')}</span></div>`
            )).join('');
            panel.style.display = 'block';
            uiManager.logAction('DEBUG', 'Opened system info panel', 'EventHandler');
        } catch (error) {
            uiManager.logAction('ERROR', `Failed to load system info: ${error}`, 'EventHandler');
        }
    };

    window.closeSystemInfo = () => {
        const panel = document.getElementById('system-info-panel');
        if (panel) panel.style.display = 'none';
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
    let dragSnapshotMods = null;
    let dragCancelRequested = false;
    let dragStartIndex = null;
    let dragTargetCandidate = null;

    const clearDragTargetCandidate = () => {
        if (dragTargetCandidate) {
            dragTargetCandidate.classList.remove('drag-target-candidate');
            dragTargetCandidate = null;
        }
    };

    const resolveTargetIndex = (listEl, evt) => {
        const childrenWithoutDragged = Array.from(listEl.children).filter((child) => child !== (evt.dragged || evt.item));
        if (!evt.related) {
            return childrenWithoutDragged.length;
        }
        const relatedIndex = childrenWithoutDragged.indexOf(evt.related);
        if (relatedIndex < 0) {
            return dragStartIndex ?? 0;
        }
        return evt.willInsertAfter ? relatedIndex + 1 : relatedIndex;
    };

    const stopFileWatcher = () => {
        if (fileCheckInterval) {
            clearInterval(fileCheckInterval);
            fileCheckInterval = null;
        }
    };

    const performFileCheck = async () => {
        if (fileCheckInProgress) return;
        if (state.isModalVisible || state.isReordering || state.isRestoring || !settingsManager.settings.noita_dir) {
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

    // --- Clean shutdown handler ---
    window.addEventListener('beforeunload', async () => {
        try {
            // Revert mod_config in dev mode
            if (settingsManager._isDevBuild && settingsManager._realNoitaDir) {
                await window.__TAURI__.core.invoke('revert_mod_config', {
                    realNoitaDir: settingsManager._realNoitaDir
                });
            }
            // Remove session lock
            await window.__TAURI__.core.invoke('remove_session_lock');
            // Flush logs
            await window.__TAURI__.core.invoke('flush_log_buffer');
        } catch (e) {
            // Best effort cleanup
        }
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

        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                if (state.isReordering) {
                    dragCancelRequested = true;
                    e.preventDefault();
                    uiManager.logAction('INFO', 'Reorder canceled. Drop to revert to original order.', 'EventHandler');
                    return;
                }

                const logModal = document.getElementById('log-modal');
                const logFullscreen = document.getElementById('log-fullscreen');
                const systemInfoPanel = document.getElementById('system-info-panel');
                const settingsPage = document.getElementById('settings-page');

                if (logFullscreen && logFullscreen.style.display !== 'none') {
                    closeLogs();
                } else if (logModal && logModal.style.display !== 'none') {
                    closeLogs();
                } else if (systemInfoPanel && systemInfoPanel.style.display !== 'none') {
                    window.closeSystemInfo();
                } else if (settingsPage && settingsPage.style.display === 'block') {
                    settingsManager.restorePreviousSettings();
                    uiManager.changeView('main');
                }
            }
        });

        await settingsManager.loadConfig();
        presetManager.loadPresets();
        initLogFilterDropdown();

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

        // Cleanup old backups on startup
        backupManager.cleanupOldBackups();

        // Start auto-backup if configured
        const backupInterval = settingsManager.settings.backup_settings?.backup_interval_minutes || 0;
        if (backupInterval > 0) {
            backupManager.startAutoBackup(backupInterval);
        }

        // Log filter event listeners (modal + fullscreen)
        ['log-filter-level', 'log-fs-filter-level'].forEach(id => {
            const el = document.getElementById(id);
            if (el) el.addEventListener('change', refreshLogs);
        });

        ['log-search', 'log-fs-search'].forEach(id => {
            const el = document.getElementById(id);
            if (el) el.addEventListener('input', refreshLogs);
        });

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
                chosenClass: 'sortable-chosen',
                dragClass: 'sortable-drag',
                fallbackClass: 'sortable-fallback',
                fallbackOnBody: true,
                forceFallback: true,
                // Reduce click/drag ambiguity:
                // - delay prevents immediate drag capture on press
                // - fallbackTolerance requires real pointer movement before drag starts
                delay: 110,
                fallbackTolerance: 7,
                touchStartThreshold: 4,
                onStart: () => {
                    uiManager.logAction('DEBUG', 'Starting mod reorder', 'EventHandler');
                    state.isReordering = true;
                    dragCancelRequested = false;
                    dragSnapshotMods = deepCopyMods(state.currentMods);
                    dragStartIndex = null;
                },
                onEnd: (evt) => {
                    clearDragTargetCandidate();
                    if (dragCancelRequested && dragSnapshotMods) {
                        state.currentMods = deepCopyMods(dragSnapshotMods);
                        state.currentPresets[state.selectedPreset] = deepCopyMods(dragSnapshotMods);
                        state.pendingReorder = false;
                        state.isReordering = false;
                        dragSnapshotMods = null;
                        dragCancelRequested = false;
                        uiManager.renderModList();
                        uiManager.updateModCount();
                        dragStartIndex = null;
                        return;
                    }

                    dragSnapshotMods = null;
                    dragStartIndex = null;

                    if (evt.oldIndex === evt.newIndex) {
                        state.pendingReorder = false;
                        state.isReordering = false;
                        uiManager.renderModList();
                        return;
                    }

                    modManager.reorderMod(evt.oldIndex, evt.newIndex);
                    setTimeout(() => {
                        modManager.finishReordering();
                    }, 100);
                },
                onChoose: (evt) => {
                    dragStartIndex = evt.oldIndex;
                    updateDragVisualNumbersByDom(list, evt.oldIndex, evt.oldIndex, evt.item);
                },
                onMove: (evt) => {
                    let targetIndex = dragStartIndex ?? 0;
                    if (list) {
                        targetIndex = resolveTargetIndex(list, evt);
                    }

                    clearDragTargetCandidate();
                    if (evt.related && evt.related !== evt.dragged && evt.related !== evt.item) {
                        dragTargetCandidate = evt.related;
                        dragTargetCandidate.classList.add('drag-target-candidate');
                    }
                    updateDragVisualNumbersByDom(
                        list,
                        evt.oldIndex ?? dragStartIndex ?? 0,
                        targetIndex,
                        evt.dragged || evt.item
                    );
                    return true;
                },
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
