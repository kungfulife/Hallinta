import {deepCopyMods, state} from './state.js';
import {updateDragVisualNumbersByDom} from './reorderUtils.js';

export function setupEventHandlers(uiManager, modManager, presetManager, settingsManager, backupManager, saveMonitorManager, galleryManager, selectEnhancer) {
    const blockWhenMonitorLocked = (actionLabel) => saveMonitorManager.isInteractionBlocked(actionLabel);
    window._hallintaSaveMonitorRunning = () => saveMonitorManager.isRunning;

    window.changeDirectory = (type) => settingsManager.changeDirectory(type);
    window.findDefaultDirectory = (type) => settingsManager.findDefaultDirectory(type);
    window.openDirectory = (type) => settingsManager.openDirectory(type);
    window.openAppSettingsFolder = () => settingsManager.openAppSettingsFolder();
    window.resetToDefaults = () => settingsManager.resetToDefaults();
    window.saveAndClose = () => settingsManager.saveAndClose();
    window.toggleDarkMode = () => uiManager.toggleDarkMode();
    window.filterMods = () => uiManager.filterMods();
    window.onPresetChange = () => {
        if (blockWhenMonitorLocked('Preset selection')) return;
        presetManager.onPresetChange();
    };
    window.renameCurrentPreset = () => {
        if (blockWhenMonitorLocked('Preset renaming')) return;
        presetManager.renameCurrentPreset();
    };
    window.deleteCurrentPreset = () => {
        if (blockWhenMonitorLocked('Preset deletion')) return;
        presetManager.deleteCurrentPreset();
    };
    window.importRegular = () => {
        if (blockWhenMonitorLocked('Mod import')) return;
        modManager.importRegular();
    };
    window.exportModList = () => {
        if (blockWhenMonitorLocked('Mod export')) return;
        modManager.exportModList();
    };
    window.createBackup = () => {
        if (blockWhenMonitorLocked('Manual backup creation')) return;
        backupManager.createBackup();
    };
    window.openRestoreUI = () => {
        if (blockWhenMonitorLocked('Backup restore')) return;
        backupManager.openRestoreUI();
    };
    window.toggleSaveMonitor = async () => {
        if (saveMonitorManager.isRunning) {
            await saveMonitorManager.stop();
        } else {
            await saveMonitorManager.start();
        }
    };
    // --- Compact Mode ---
    const applyCompactMode = (active) => {
        state.compactMode = !!active;
        document.body.classList.toggle('compact-mode', !!active);

        const checkbox = document.getElementById('compact-mode-checkbox');
        if (checkbox) checkbox.checked = !!active;

        // Close context menu
        const menu = document.getElementById('mod-context-menu');
        if (menu) menu.style.display = 'none';

        // Sync sortable disabled state
        const sortable = window.__hallintaSortable;
        if (sortable && typeof sortable.option === 'function') {
            sortable.option('disabled', !!(active || saveMonitorManager.isRunning));
        }
    };

    const toggleCompactMode = async () => {
        const newState = !state.compactMode;

        try {
            const appWindow = window.__TAURI__.webviewWindow.getCurrentWebviewWindow();
            const LogicalSize = window.__TAURI__.dpi.LogicalSize;

            if (newState) {
                // Save current size before compacting
                const currentSize = await appWindow.innerSize();
                if (currentSize) {
                    state.normalModeWindowSize = { width: currentSize.width, height: currentSize.height };
                }

                applyCompactMode(true);

                await appWindow.setMinSize(new LogicalSize(380, 340));
                await appWindow.setMaxSize(new LogicalSize(600, 500));
                await appWindow.setSize(new LogicalSize(480, 400));
                await appWindow.center();
            } else {
                applyCompactMode(false);

                await appWindow.setMaxSize(null);
                await appWindow.setMinSize(new LogicalSize(1050, 800));

                const saved = state.normalModeWindowSize;
                const restoreWidth = saved ? Math.max(saved.width, 1050) : 1100;
                const restoreHeight = saved ? Math.max(saved.height, 800) : 800;
                await appWindow.setSize(new LogicalSize(restoreWidth, restoreHeight));
                await appWindow.center();

                state.normalModeWindowSize = null;
            }
        } catch (e) {
            applyCompactMode(newState);
            uiManager.logAction('WARN', `Could not resize window for compact mode: ${e}`, 'EventHandler');
        }

        // Persist compact_mode setting
        settingsManager.settings.compact_mode = newState;
        try {
            const settingsToSave = settingsManager.getSettingsForPersistence();
            await window.__TAURI__.core.invoke('save_settings', { settings: settingsToSave });
        } catch (e) {
            uiManager.logAction('WARN', `Could not persist compact mode setting: ${e}`, 'EventHandler');
        }

        uiManager.logAction('INFO', `Compact mode ${newState ? 'enabled' : 'disabled'}`, 'EventHandler');
    };

    window.toggleCompactMode = toggleCompactMode;

    window.updateLogLevelColor = () => settingsManager.updateLogLevelSelectColor();
    window.exportPresets = () => {
        if (blockWhenMonitorLocked('Preset export')) return;
        presetManager.exportPresets();
    };
    window.importPresets = () => {
        if (blockWhenMonitorLocked('Preset import')) return;
        presetManager.importPresets();
    };

    // Preset Vault handlers
    window.showModListView = () => {
        const button = document.getElementById('header-combined-button');
        const isInSettings = button && button.textContent === 'Cancel';

        if (isInSettings) {
            state.galleryView = false;
            settingsManager.restorePreviousSettings();
            uiManager.changeView('main');
            return;
        }

        if (galleryManager.isGalleryOpen()) {
            galleryManager.closeGallery();
            return;
        }

        uiManager.changeView('main');
    };
    window.showLoadoutView = window.showModListView;

    window.showPresetVaultView = () => {
        if (blockWhenMonitorLocked('Preset Vault access')) return;
        if (galleryManager.isGalleryOpen()) {
            return;
        }

        // Close settings if open
        const button = document.getElementById('header-combined-button');
        if (button && button.textContent === 'Cancel') {
            settingsManager.restorePreviousSettings();
        }
        galleryManager.openGallery();
    };

    // Backward compatibility for older inline hooks
    window.toggleGalleryView = () => window.showPresetVaultView();
    window.filterGallery = () => {
        if (blockWhenMonitorLocked('Preset Vault filtering')) return;
        galleryManager.filterAndRender();
    };
    window.refreshGallery = () => {
        if (blockWhenMonitorLocked('Preset Vault refresh')) return;
        galleryManager.refreshGallery();
    };
    window.downloadByShareLink = () => {
        if (blockWhenMonitorLocked('Preset import by link')) return;
        galleryManager.downloadByShareLink();
    };
    window.showVaultHelp = () => {
        uiManager.showInfoModal(`
            <h3>Hosting a Preset Catalog</h3>
            <p>The Preset Vault loads presets from a single JSON file hosted at any URL.</p>
            <p><strong>Catalog JSON structure:</strong></p>
            <pre style="background:var(--bg-color);border:1px solid var(--border-color);border-radius:6px;padding:0.75em;font-size:0.85em;overflow-x:auto;white-space:pre">{
  "catalog_version": "1.0",
  "last_updated": "2025-01-01",
  "presets": [
    {
      "id": "unique-id",
      "name": "Preset Name",
      "description": "What this preset does",
      "author": "Author Name",
      "tags": ["tag1", "tag2"],
      "mod_count": 10,
      "version": "1.0",
      "checksum": "sha256-hash",
      "download_url": "https://example.com/preset.json"
    }
  ]
}</pre>
            <p>Each <code>download_url</code> points to a standard Hallinta preset export file (created via Export Presets).</p>
            <p>The catalog can be hosted anywhere: a web server, GitHub Pages, a local network, or any service that serves JSON files.</p>
        `, 'Got it');
    };

    window.detectSteamPath = async () => {
        if (blockWhenMonitorLocked('Steam path detection')) return;
        try {
            const steamPath = await window.__TAURI__.core.invoke('detect_steam_path');
            const input = document.getElementById('gallery-steam-path');
            if (input) input.value = steamPath;
            uiManager.logAction('INFO', `Detected Steam path: ${steamPath}`, 'EventHandler');
        } catch (error) {
            uiManager.logAction('WARN', `Could not detect Steam path: ${error}`, 'EventHandler');
        }
    };
    window.toggleMod = () => {
        if (blockWhenMonitorLocked('Mod toggling')) return;
        uiManager.toggleMod();
    };
    window.reorderMod = () => {
        if (blockWhenMonitorLocked('Mod reordering')) return;
        uiManager.reorderMod();
    };
    window.deleteMod = () => {
        if (blockWhenMonitorLocked('Mod deletion')) return;
        uiManager.deleteMod();
    };
    window.openWorkshop = () => {
        if (blockWhenMonitorLocked('Workshop link opening')) return;
        uiManager.openWorkshop();
    };
    window.copyWorkshopLink = () => {
        if (blockWhenMonitorLocked('Workshop link copy')) return;
        uiManager.copyWorkshopLink();
    };

    window.openSystemInfo = async () => {
        const panel = document.getElementById('system-info-panel');
        const body = document.getElementById('system-info-body');
        if (!panel || !body) return;

        try {
            const info = await window.__TAURI__.core.invoke('get_system_info');
            const configuredNoitaDir = (settingsManager._isDevBuild && settingsManager._realNoitaDir)
                ? settingsManager._realNoitaDir
                : settingsManager.settings.noita_dir;
            const rows = [
                ['App Version', info.app_version],
                ['Build Profile', info.build_profile],
                ['Build Target Platform', info.build_target],
                ['Runtime OS', info.os],
                ['OS Family', info.os_family],
                ['CPU Architecture', info.arch],
                ['Logical CPU Cores', String(info.logical_cpu_cores ?? '')],
                ['Rust Compiler', info.rust_version],
                ['Cargo', info.cargo_version],
                ['Tauri', info.tauri_version],
                ['App Data Directory', info.app_data_dir],
                ['Executable Directory', info.executable_dir],
                ['Local Time', info.local_time],
                ['UTC Time', info.utc_time],
                ['Noita Save Directory (Configured)', configuredNoitaDir || '(not set)'],
                ['Entangled Worlds Directory (Configured)', settingsManager.settings.entangled_dir || '(not set)'],
                ['Startup System Logging', settingsManager.settings.log_settings?.collect_system_info ? 'Enabled' : 'Disabled']
            ];
            body.innerHTML = rows.map(([label, value]) => (
                `<div class="system-info-row"><strong>${window.logUtils.escapeHtml(label)}</strong><span>${window.logUtils.escapeHtml(value || '')}</span></div>`
            )).join('');
            const openSourcePanel = document.getElementById('open-source-panel');
            if (openSourcePanel) openSourcePanel.style.display = 'none';
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

    window.openOpenSourceLibraries = async () => {
        const panel = document.getElementById('open-source-panel');
        const body = document.getElementById('open-source-body');
        if (!panel || !body) return;

        try {
            const libraries = await window.__TAURI__.core.invoke('get_open_source_libraries');
            if (!Array.isArray(libraries) || libraries.length === 0) {
                body.innerHTML = '<div class="system-info-row"><strong>Libraries</strong><span>No library data available.</span></div>';
            } else {
                body.innerHTML = libraries.map((library) => {
                    const name = window.logUtils.escapeHtml(library.name || 'Unknown');
                    const version = window.logUtils.escapeHtml(library.version || 'Unknown');
                    const purpose = window.logUtils.escapeHtml(library.purpose || '');
                    const homepageText = window.logUtils.escapeHtml(library.homepage || '');
                    const homepageHref = encodeURI(library.homepage || '');
                    return (
                        `<div class="system-info-row">` +
                        `<strong>${name} v${version}</strong>` +
                        `<span>${purpose}<br><a class="system-info-link" href="${homepageHref}" target="_blank" rel="noopener noreferrer">${homepageText}</a></span>` +
                        `</div>`
                    );
                }).join('');
            }

            const systemInfoPanel = document.getElementById('system-info-panel');
            if (systemInfoPanel) systemInfoPanel.style.display = 'none';
            panel.style.display = 'block';
            uiManager.logAction('DEBUG', 'Opened open source libraries panel', 'EventHandler');
        } catch (error) {
            uiManager.logAction('ERROR', `Failed to load open source libraries: ${error}`, 'EventHandler');
        }
    };

    window.closeOpenSourceLibraries = () => {
        const panel = document.getElementById('open-source-panel');
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
            if (state.galleryView) {
                uiManager.changeView('gallery');
            } else {
                uiManager.changeView('main');
            }
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
        if (
            saveMonitorManager.isRunning ||
            state.isModalVisible ||
            state.isReordering ||
            state.isRestoring ||
            !settingsManager.settings.noita_dir
        ) {
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

    // --- Clean shutdown handler via Tauri close interception ---
    const appWindow = window.__TAURI__.webviewWindow.getCurrentWebviewWindow();
    appWindow.onCloseRequested(async (event) => {
        event.preventDefault();

        if (saveMonitorManager.isRunning) {
            const shouldClose = await saveMonitorManager.confirmStopOnClose();
            if (!shouldClose) return;

            try { await saveMonitorManager.takeExitSnapshot(); } catch (e) { /* best effort */ }
            await saveMonitorManager.stop();
        }

        // Normal cleanup
        try {
            uiManager.logAction('INFO', 'Application closing', 'App');
            if (settingsManager._isDevBuild && settingsManager._realNoitaDir) {
                await window.__TAURI__.core.invoke('revert_mod_config', {
                    realNoitaDir: settingsManager._realNoitaDir
                });
            }
            await window.__TAURI__.core.invoke('remove_session_lock');
            await window.__TAURI__.core.invoke('flush_log_buffer');
        } catch (e) { /* best effort */ }

        appWindow.destroy();
    });

    document.addEventListener('DOMContentLoaded', async () => {

        uiManager.logAction('INFO', 'Application starting', 'App');
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
                if (state.compactMode || saveMonitorManager.isRunning) {
                    event.preventDefault();
                    return;
                }
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
                    uiManager.logAction('DEBUG', 'Escape keybind triggered: cancel active reorder', 'EventHandler');
                    uiManager.logAction('INFO', 'Reorder canceled. Drop to revert to original order.', 'EventHandler');
                    return;
                }

                const systemInfoPanel = document.getElementById('system-info-panel');
                const openSourcePanel = document.getElementById('open-source-panel');
                const settingsPage = document.getElementById('settings-page');

                if (systemInfoPanel && systemInfoPanel.style.display !== 'none') {
                    uiManager.logAction('DEBUG', 'Escape keybind triggered: close system info panel', 'EventHandler');
                    window.closeSystemInfo();
                } else if (openSourcePanel && openSourcePanel.style.display !== 'none') {
                    uiManager.logAction('DEBUG', 'Escape keybind triggered: close open source panel', 'EventHandler');
                    window.closeOpenSourceLibraries();
                } else if (galleryManager.isGalleryOpen()) {
                    uiManager.logAction('DEBUG', 'Escape keybind triggered: return from Preset Vault to Mod List', 'EventHandler');
                    galleryManager.closeGallery();
                } else if (settingsPage && settingsPage.style.display === 'block') {
                    uiManager.logAction('DEBUG', 'Escape keybind triggered: cancel settings changes', 'EventHandler');
                    settingsManager.restorePreviousSettings();
                    if (state.galleryView) {
                        uiManager.changeView('gallery');
                    } else {
                        uiManager.changeView('main');
                    }
                }
            }
        });

        await settingsManager.loadConfig();

        // Apply compact mode from persisted settings before rendering
        if (settingsManager.settings.compact_mode) {
            applyCompactMode(true);
            try {
                const appWindow = window.__TAURI__.webviewWindow.getCurrentWebviewWindow();
                const LogicalSize = window.__TAURI__.dpi.LogicalSize;
                const currentSize = await appWindow.innerSize();
                if (currentSize) {
                    state.normalModeWindowSize = { width: currentSize.width, height: currentSize.height };
                }
                await appWindow.setMinSize(new LogicalSize(380, 340));
                await appWindow.setMaxSize(new LogicalSize(600, 500));
                await appWindow.setSize(new LogicalSize(480, 400));
                await appWindow.center();
            } catch (e) {
                uiManager.logAction('WARN', `Could not resize window for compact mode on startup: ${e}`, 'EventHandler');
            }
        }

        presetManager.loadPresets();
        if (selectEnhancer) {
            selectEnhancer.enhance('mod-filter-mode', { variant: 'header' });
            selectEnhancer.enhance('preset-dropdown', { variant: 'header' });
            selectEnhancer.enhance('log-level-select', { variant: 'settings' });
            selectEnhancer.sync('mod-filter-mode');
            selectEnhancer.sync('preset-dropdown');
            selectEnhancer.sync('log-level-select');
        }
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

        const startInMonitorMode = !!settingsManager.settings.save_monitor_settings?.start_in_monitor_mode;
        if (startInMonitorMode) {
            await saveMonitorManager.start(false, { startup: true });
        }

        uiManager.logAction('INFO', 'Application ready', 'App');

        const list = document.getElementById('mod-list');
        if (list) {
            const sortableInstance = new Sortable(list, {
                animation: 150,
                ghostClass: 'sortable-ghost',
                chosenClass: 'sortable-chosen',
                dragClass: 'sortable-drag',
                fallbackClass: 'sortable-fallback',
                fallbackOnBody: true,
                forceFallback: true,
                disabled: !!(state.compactMode || saveMonitorManager.isRunning),
                // Reduce click/drag ambiguity:
                // - delay prevents immediate drag capture on press (lower = faster drag start)
                // - fallbackTolerance requires real pointer movement before drag starts
                // - delayOnTouchOnly limits delay to touch devices, keeping mouse drag snappy
                delay: 80,
                delayOnTouchOnly: true,
                fallbackTolerance: 5,
                touchStartThreshold: 4,
                onStart: () => {
                    if (state.compactMode || saveMonitorManager.isRunning) {
                        return;
                    }
                    uiManager.logAction('DEBUG', 'Starting mod reorder', 'EventHandler');
                    state.isReordering = true;
                    dragCancelRequested = false;
                    dragSnapshotMods = deepCopyMods(state.currentMods);
                    dragStartIndex = null;
                },
                onEnd: (evt) => {
                    if (state.compactMode || saveMonitorManager.isRunning) {
                        state.isReordering = false;
                        state.pendingReorder = false;
                        return;
                    }
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
                    if (state.compactMode || saveMonitorManager.isRunning) return;
                    dragStartIndex = evt.oldIndex;
                    updateDragVisualNumbersByDom(list, evt.oldIndex, evt.oldIndex, evt.item);
                },
                onMove: (evt) => {
                    if (state.compactMode || saveMonitorManager.isRunning) return false;
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
            window.__hallintaSortable = sortableInstance;
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
