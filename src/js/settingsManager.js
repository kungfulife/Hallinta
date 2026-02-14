import { buildPresetsForSave, deepCopyMods, state } from './state.js';

export class SettingsManager {
    constructor(modManager, uiManager) {
        this.modManager = modManager;
        this.uiManager = uiManager;
        this.settings = {
            noita_dir: '',
            entangled_dir: '',
            dark_mode: false,
            selected_preset: 'Default',
            version: '',
            log_settings: {
                max_log_files: 50,
                max_log_size_mb: 10,
                log_level: 'INFO',
                auto_save: true
            },
            backup_settings: {
                auto_delete_days: 30,
                backup_interval_minutes: 0
            },
            save_monitor_settings: {
                interval_minutes: 15,
                max_snapshots_per_preset: 10,
                include_entangled: false
            }
        };
        this.previousSettings = null;
        this.previousIsDarkMode = false;
        this._isDevBuild = false;
        this._realNoitaDir = '';
        this._devSaveDir = '';
        this._previousRealNoitaDir = '';
    }

    getSettingsForPersistence() {
        const settingsToSave = { ...this.settings };
        if (this._isDevBuild && this._devSaveDir) {
            settingsToSave.noita_dir = this._realNoitaDir;
        }
        return settingsToSave;
    }

    async loadConfig() {
        if (!window.__TAURI__?.core) {
            this.logAction('ERROR', 'Tauri is not initialized. Application may not function correctly.')
            return false;
        }

        let hasError = false;
        try {
            let settings, presets, version;
            try {
                version = await window.__TAURI__.core.invoke('get_version');
                settings = await window.__TAURI__.core.invoke('load_settings');
                presets = await window.__TAURI__.core.invoke('load_presets');
            } catch (error) {
                const errorMsg = error.message || error || 'Unknown error';
                const isUpgradeError = errorMsg.includes('upgrade backup') || errorMsg.includes('Failed to create');
                if (isUpgradeError) {
                    this.logAction('ERROR', `Upgrade preflight backup failed: ${errorMsg}. Normal operations blocked until resolved.`);
                } else {
                    this.logAction('ERROR', `Error loading configuration: ${errorMsg}. Using defaults.`);
                }
                hasError = true;
                settings = {
                    noita_dir: '',
                    entangled_dir: '',
                    dark_mode: false,
                    selected_preset: 'Default',
                    version: version || 'unknown',
                    log_settings: {
                        max_log_files: 50,
                        max_log_size_mb: 10,
                        log_level: 'INFO',
                        auto_save: true
                    },
                    backup_settings: {
                        auto_delete_days: 30,
                        backup_interval_minutes: 0
                    },
                    save_monitor_settings: {
                        interval_minutes: 15,
                        max_snapshots_per_preset: 10,
                        include_entangled: false
                    }
                };

                // Try to get default paths, log warnings if not found
                try {
                    settings.noita_dir = await window.__TAURI__.core.invoke('get_noita_save_path');
                } catch (pathError) {
                    this.logAction('WARN', `Noita directory not found automatically: ${pathError.message}`);
                    settings.noita_dir = '';
                }

                try {
                    settings.entangled_dir = await window.__TAURI__.core.invoke('get_entangled_worlds_config_path');
                } catch (pathError) {
                    this.logAction('WARN', `Entangled Worlds directory not found (optional): ${pathError.message}`);
                    settings.entangled_dir = '';
                }

                presets = { "Default": [] };
                await this.saveDefaults(settings, presets);
                this.logAction('INFO', 'Created default configuration');
            }

            // Apply log level early so dev mode messages are filtered correctly
            if (settings.log_settings?.log_level) {
                this.settings.log_settings.log_level = settings.log_settings.log_level;
            }

            this._isDevBuild = await window.__TAURI__.core.invoke('is_dev_build');

            // Check for stale session lock (previous crash detection)
            try {
                const staleLock = await window.__TAURI__.core.invoke('check_session_lock');
                if (staleLock && staleLock.dev_mode_active) {
                    this.logAction('WARN', 'Stale session lock detected from previous session');
                    const cacheExists = await window.__TAURI__.core.invoke('check_mod_config_cache_exists');
                    if (cacheExists && staleLock.original_mod_config_path) {
                        // Extract the directory from the cached path (it stores full path to mod_config.xml)
                        const realDir = staleLock.original_mod_config_path.replace(/[/\\]mod_config\.xml$/, '');
                        await new Promise((resolve) => {
                            this.uiManager.showConfirmModal(
                                'Previous session did not shut down cleanly. Revert mod_config.xml to original state?', {
                                    confirmText: 'Revert to Original',
                                    cancelText: 'Keep Current',
                                    onConfirm: async () => {
                                        try {
                                            await window.__TAURI__.core.invoke('revert_mod_config', { realNoitaDir: realDir });
                                            this.logAction('INFO', 'Reverted mod_config.xml to original state');
                                        } catch (e) {
                                            this.logAction('ERROR', `Failed to revert mod_config.xml: ${e}`);
                                        }
                                        resolve();
                                    },
                                    onCancel: () => {
                                        this.logAction('INFO', 'Keeping current mod_config.xml');
                                        resolve();
                                    },
                                    isImportant: true
                                }
                            );
                        });
                    }
                    // Remove the stale lock regardless
                    await window.__TAURI__.core.invoke('remove_session_lock');
                } else if (staleLock) {
                    await window.__TAURI__.core.invoke('remove_session_lock');
                }
            } catch (lockError) {
                this.logAction('WARN', `Error checking session lock: ${lockError}`);
            }

            if (this._isDevBuild) {
                try {
                    const devDir = await window.__TAURI__.core.invoke('get_dev_save_dir', {
                        sourceNoitaDir: settings.noita_dir
                    });
                    this._realNoitaDir = settings.noita_dir;
                    this._devSaveDir = devDir;
                    settings.noita_dir = devDir;
                    this.logAction('DEV', `Using dev_data directory for mod_config.xml: ${devDir}`);
                    if (this._realNoitaDir) {
                        this.logAction('DEV', `Real Noita directory preserved: ${this._realNoitaDir}`);

                        // Cache and overwrite the real mod_config.xml with dev version
                        try {
                            await window.__TAURI__.core.invoke('cache_and_overwrite_mod_config', {
                                realNoitaDir: this._realNoitaDir,
                                devDataDir: devDir
                            });
                            this.logAction('DEV', 'Cached original mod_config.xml and overwrote with dev version');
                        } catch (overwriteError) {
                            this.logAction('WARN', `DEV MODE: Could not overwrite mod_config.xml: ${overwriteError}`);
                        }
                    }
                } catch (devError) {
                    this.logAction('WARN', `Could not set up dev save directory: ${devError}`);
                }
            }

            // Create session lock
            try {
                const originalConfigPath = this._isDevBuild && this._realNoitaDir
                    ? `${this._realNoitaDir}/mod_config.xml`
                    : '';
                await window.__TAURI__.core.invoke('create_session_lock', {
                    devModeActive: this._isDevBuild && !!this._devSaveDir,
                    originalModConfigPath: originalConfigPath
                });
                this.logAction('DEBUG', 'Session lock created');
            } catch (lockError) {
                this.logAction('WARN', `Could not create session lock: ${lockError}`);
            }

            this.applyConfig(settings, presets);
            this.logAction('DEBUG', `Startup selected preset from settings: ${state.selectedPreset}`);
            const versionElement = document.getElementById('app-version');
            if (versionElement) {
                versionElement.textContent = settings.version;
            }

            if (settings.noita_dir) {
                const configPath = `${settings.noita_dir}/mod_config.xml`;
                const fileExists = await window.__TAURI__.core.invoke('check_file_exists', { path: configPath });
                if (fileExists) {
                    state.lastModifiedTime = await window.__TAURI__.core.invoke('get_file_modified_time', { filePath: configPath });
                    await this.modManager.loadModConfigFromDirectory(settings.noita_dir, { startupSync: true });
                } else {
                    this.logAction('ERROR', 'Noita save directory does not contain mod_config.xml');
                    hasError = true;
                }
            } else {
                this.logAction('ERROR', 'Noita save directory not set. Please set it in settings.');
                hasError = true;
            }

            if (!hasError) {
                this.logAction('INFO', 'Configuration loaded successfully');
            }
            return !hasError;
        } catch (error) {
            this.logAction('ERROR', `Critical error in loadConfig: ${error.message}`);
            return false;
        }
    }

    async saveDefaults(settings, presets) {
        try {
            await window.__TAURI__.core.invoke('save_settings', { settings });
            await window.__TAURI__.core.invoke('save_presets', { presets });
        } catch (error) {
            this.logAction('ERROR', `Error saving default settings: ${error.message}`);
        }
    }

    applyConfig(settings, presets) {
        this.settings = settings;
        // Ensure backup_settings defaults
        if (!this.settings.backup_settings) {
            this.settings.backup_settings = {
                auto_delete_days: 30,
                backup_interval_minutes: 0
            };
        }
        // Ensure save_monitor_settings defaults
        if (!this.settings.save_monitor_settings) {
            this.settings.save_monitor_settings = {
                interval_minutes: 15,
                max_snapshots_per_preset: 10,
                include_entangled: false
            };
        }

        state.currentPresets = Object.keys(presets).reduce((acc, presetName) => {
            acc[presetName] = presets[presetName].map(mod => ({
                name: mod.name,
                enabled: mod.enabled,
                workshopId: mod.workshop_id || '0',
                settingsFoldOpen: mod.settings_fold_open || false,
                index: 0
            }));
            return acc;
        }, {});
        state.selectedPreset = settings.selected_preset || 'Default';
        if (!state.currentPresets[state.selectedPreset]) {
            state.selectedPreset = 'Default';
            if (!state.currentPresets['Default']) {
                state.currentPresets['Default'] = [];
            }
        }

        state.isDarkMode = settings.dark_mode;
        const noitaDirElement = document.getElementById('noita-dir');
        const entangledDirElement = document.getElementById('entangled-dir');
        const darkModeElement = document.getElementById('dark-mode-checkbox');
        const logLevelSelect = document.getElementById('log-level-select');
        const autoDeleteDaysInput = document.getElementById('auto-delete-days');
        const backupIntervalInput = document.getElementById('backup-interval');
        const monitorIntervalInput = document.getElementById('monitor-interval');
        const monitorMaxSnapshotsInput = document.getElementById('monitor-max-snapshots');

        // In dev mode, show the real Noita path in the input field (not dev_data)
        if (noitaDirElement) {
            noitaDirElement.value = (this._isDevBuild && this._realNoitaDir) ? this._realNoitaDir : settings.noita_dir;
        }
        if (entangledDirElement) entangledDirElement.value = settings.entangled_dir;
        if (darkModeElement) darkModeElement.checked = state.isDarkMode;
        if (logLevelSelect) logLevelSelect.value = settings.log_settings.log_level || 'INFO';
        if (autoDeleteDaysInput) autoDeleteDaysInput.value = this.settings.backup_settings.auto_delete_days;
        if (backupIntervalInput) backupIntervalInput.value = this.settings.backup_settings.backup_interval_minutes;
        if (monitorIntervalInput) monitorIntervalInput.value = this.settings.save_monitor_settings?.interval_minutes ?? 15;
        if (monitorMaxSnapshotsInput) monitorMaxSnapshotsInput.value = this.settings.save_monitor_settings?.max_snapshots_per_preset ?? 10;

        // Show/hide dev data directory section
        this._updateDevDataSection();

        this.uiManager.applyDarkMode();
    }

    _updateDevDataSection() {
        const devSection = document.getElementById('dev-data-section');
        const devDirInput = document.getElementById('dev-data-dir');
        if (devSection) {
            if (this._isDevBuild && this._devSaveDir) {
                devSection.style.display = 'block';
                if (devDirInput) devDirInput.value = this._devSaveDir;
            } else {
                devSection.style.display = 'none';
            }
        }
    }

    async saveAndClose() {
        try {
            const noitaDirElement = document.getElementById('noita-dir');
            const entangledDirElement = document.getElementById('entangled-dir');
            const logLevelSelect = document.getElementById('log-level-select');
            const autoDeleteDaysInput = document.getElementById('auto-delete-days');
            const backupIntervalInput = document.getElementById('backup-interval');
            const monitorIntervalInput = document.getElementById('monitor-interval');
            const monitorMaxSnapshotsInput = document.getElementById('monitor-max-snapshots');

            this.settings.noita_dir = noitaDirElement ? noitaDirElement.value : '';
            this.settings.entangled_dir = entangledDirElement ? entangledDirElement.value : '';
            this.settings.dark_mode = state.isDarkMode;
            this.settings.selected_preset = state.selectedPreset;
            if (logLevelSelect) this.settings.log_settings.log_level = logLevelSelect.value;
            if (autoDeleteDaysInput) {
                const days = parseInt(autoDeleteDaysInput.value);
                this.settings.backup_settings.auto_delete_days = isNaN(days) ? 30 : days;
            }
            if (backupIntervalInput) {
                const mins = parseInt(backupIntervalInput.value);
                this.settings.backup_settings.backup_interval_minutes = isNaN(mins) ? 0 : mins;
            }
            if (!this.settings.save_monitor_settings) {
                this.settings.save_monitor_settings = {};
            }
            if (monitorIntervalInput) {
                const val = parseInt(monitorIntervalInput.value);
                this.settings.save_monitor_settings.interval_minutes = isNaN(val) || val < 1 ? 15 : val;
            }
            if (monitorMaxSnapshotsInput) {
                const val = parseInt(monitorMaxSnapshotsInput.value);
                this.settings.save_monitor_settings.max_snapshots_per_preset = isNaN(val) || val < 1 ? 10 : val;
            }
            this.settings.version = await window.__TAURI__.core.invoke('get_version').catch(() => 'unknown');

            // In dev mode, ensure we operate against dev_data regardless of DOM edits
            if (this._isDevBuild && this._devSaveDir) {
                this.settings.noita_dir = this._devSaveDir;
            }

            if (this.settings.noita_dir) {
                const configPath = `${this.settings.noita_dir}/mod_config.xml`;
                const fileExists = await window.__TAURI__.core.invoke('check_file_exists', { path: configPath });
                if (!fileExists) {
                    this.logAction('ERROR', 'Cannot save: mod_config.xml not found in Noita directory');
                    return;
                }
            }

            // Detect if noita directory changed while in settings
            const previousNoitaDir = this.previousSettings?.noita_dir || '';
            const directoryChanged = this.settings.noita_dir && this.settings.noita_dir !== previousNoitaDir;

            if (window.__TAURI__?.core) {
                if (directoryChanged && previousNoitaDir) {
                    // Confirm directory change with the user before proceeding
                    const confirmed = await new Promise((resolve) => {
                        this.uiManager.showConfirmModal(
                            'Noita directory has changed. Current mod list will be replaced with mods from the new directory. Continue?', {
                                confirmText: 'Switch Directory',
                                cancelText: 'Cancel',
                                onConfirm: () => resolve(true),
                                onCancel: () => resolve(false)
                            }
                        );
                    });

                    if (!confirmed) {
                        // Revert the directory change
                        this.settings.noita_dir = previousNoitaDir;
                        const noitaDirElement = document.getElementById('noita-dir');
                        if (noitaDirElement) {
                            noitaDirElement.value = (this._isDevBuild && this._realNoitaDir)
                                ? this._previousRealNoitaDir
                                : previousNoitaDir;
                        }
                        if (this._isDevBuild) {
                            this._realNoitaDir = this._previousRealNoitaDir;
                        }
                        this.logAction('INFO', 'Directory change cancelled');
                        return;
                    }
                }

                const presetsForSave = buildPresetsForSave(state.currentPresets);

                const settingsToSave = this.getSettingsForPersistence();
                await window.__TAURI__.core.invoke('save_settings', { settings: settingsToSave });
                await window.__TAURI__.core.invoke('save_presets', { presets: presetsForSave });

                if (directoryChanged) {
                    // Directory changed — load mods from new directory instead of
                    // writing old mods to it (which would overwrite the new file)
                    await this.modManager.loadModConfigFromDirectory(this.settings.noita_dir);
                    this.logAction('INFO', `Noita directory changed, loaded mods from: ${this.settings.noita_dir}`);
                } else {
                    await this.modManager.saveModConfigToFile();
                }
                this.logAction('INFO', 'Configuration saved successfully');
            }

            this.uiManager.changeView('main');
        } catch (error) {
            this.logAction('ERROR', `Critical error during save: ${error.message}`);
            this.uiManager.changeView('main');
        }
    }

    async changeDirectory(type) {
        try {
            if (window.__TAURI__?.dialog) {
                const selected = await window.__TAURI__.dialog.open({
                    directory: '',
                    multiple: false
                });
                if (selected) {
                    if (this._isDevBuild && type === 'noita' && this._devSaveDir) {
                        this._realNoitaDir = selected;
                        const dirElement = document.getElementById('noita-dir');
                        if (dirElement) dirElement.value = selected;
                        this.logAction('DEV', `Updated production Noita directory to: ${selected}`);
                        this.logAction('DEV', `Mod operations still use dev_data directory`);
                    } else {
                        const dirElement = document.getElementById(`${type}-dir`);
                        if (dirElement) dirElement.value = selected;
                        this.settings[`${type}_dir`] = selected;
                        this.logAction('DEBUG', `Selected directory for ${type}: ${selected}`);
                        if (type === 'noita') {
                            await this.modManager.loadModConfigFromDirectory(selected);
                        }
                    }
                }
            }
        } catch (error) {
            this.logAction('ERROR', `Error selecting directory: ${error.message}`);
        }
    }

    async findDefaultDirectory(type) {
        try {
            let defaultPath = '';
            let commandName = '';

            if (type === 'noita') {
                commandName = 'get_noita_save_path';
            } else if (type === 'entangled') {
                commandName = 'get_entangled_worlds_config_path';
            }

            if (commandName) {
                try {
                    defaultPath = await window.__TAURI__.core.invoke(commandName);

                    if (this._isDevBuild && type === 'noita' && this._devSaveDir) {
                        this._realNoitaDir = defaultPath;
                        const dirElement = document.getElementById('noita-dir');
                        if (dirElement) dirElement.value = defaultPath;
                        this.logAction('DEV', `Updated production Noita directory to: ${defaultPath}`);
                        this.logAction('DEV', `Mod operations still use dev_data directory`);
                    } else {
                        const dirElement = document.getElementById(`${type}-dir`);
                        if (dirElement) dirElement.value = defaultPath;
                        this.settings[`${type}_dir`] = defaultPath;
                        this.logAction('INFO', `Found default ${type} directory: ${defaultPath}`);

                        if (type === 'noita') {
                            await this.modManager.loadModConfigFromDirectory(defaultPath);
                        }
                    }
                } catch (pathError) {
                    if (type === 'entangled') {
                        this.logAction('WARN', `Default Entangled Worlds directory not found (optional): ${pathError.message}`);
                    } else {
                        this.logAction('ERROR', `Default ${type} directory not found: ${pathError.message}`);
                    }
                }
            }
        } catch (error) {
            this.logAction('ERROR', `Error finding default directory: ${error.message}`);
        }
    }

    async openDirectory(type) {
        let directory;
        if (type === 'dev-data') {
            directory = this._devSaveDir;
        } else {
            const dirElement = document.getElementById(`${type}-dir`);
            directory = dirElement ? dirElement.value : '';
        }

        if (!directory) {
            this.logAction('ERROR', `No ${type} directory set`);
            return;
        }

        try {
            await window.__TAURI__.core.invoke('open_directory', { directory });
            this.logAction('DEBUG', `Opened ${type} directory: ${directory}`);
        } catch (error) {
            this.logAction('ERROR', `Error opening directory: ${error.message}`);
        }
    }

    async openAppSettingsFolder() {
        try {
            const settingsDir = await window.__TAURI__.core.invoke('get_app_settings_dir');
            await window.__TAURI__.core.invoke('open_directory', { directory: settingsDir });
            this.logAction('DEBUG', `Opened directory: ${settingsDir}`);
        } catch (error) {
            this.logAction('ERROR', `Error opening directory: ${error.message}`);
        }
    }

    async resetToDefaults() {
        try {
            if (!window.__TAURI__?.core) {
                throw new Error('Tauri is not initialized');
            }

            let defaultNoitaDir = '';
            let defaultEntangledDir = '';

            // Try to get default Noita directory, log warning if not found
            try {
                defaultNoitaDir = await window.__TAURI__.core.invoke('get_noita_save_path');
            } catch (pathError) {
                this.logAction('WARN', `Default Noita directory not found: ${pathError.message}`);
            }

            // Try to get default Entangled Worlds directory, log warning if not found (optional)
            try {
                defaultEntangledDir = await window.__TAURI__.core.invoke('get_entangled_worlds_config_path');
            } catch (pathError) {
                this.logAction('WARN', `Default Entangled Worlds directory not found (optional): ${pathError.message}`);
            }

            // In dev mode, update the real path but keep using dev_data
            let effectiveNoitaDir = defaultNoitaDir;
            if (this._isDevBuild && this._devSaveDir) {
                this._realNoitaDir = defaultNoitaDir;
                effectiveNoitaDir = this._devSaveDir;
                this.logAction('DEV', `Reset real Noita directory to: ${defaultNoitaDir}`);
            }

            this.settings = {
                noita_dir: effectiveNoitaDir,
                entangled_dir: defaultEntangledDir,
                dark_mode: false,
                selected_preset: 'Default',
                version: await window.__TAURI__.core.invoke('get_version').catch(() => 'unknown'),
                log_settings: {
                    max_log_files: 50,
                    max_log_size_mb: 10,
                    log_level: 'INFO',
                    auto_save: true
                },
                backup_settings: {
                    auto_delete_days: 30,
                    backup_interval_minutes: 0
                },
                save_monitor_settings: {
                    interval_minutes: 15,
                    max_snapshots_per_preset: 10,
                    include_entangled: false
                }
            };
            state.isDarkMode = false;
            state.selectedPreset = 'Default';
            if (!state.currentPresets['Default']) {
                state.currentPresets['Default'] = [];
            }
            state.currentMods = deepCopyMods(state.currentPresets['Default']);
            const noitaDirElement = document.getElementById('noita-dir');
            const entangledDirElement = document.getElementById('entangled-dir');
            const darkModeElement = document.getElementById('dark-mode-checkbox');
            const logLevelSelect = document.getElementById('log-level-select');
            const autoDeleteDaysInput = document.getElementById('auto-delete-days');
            const backupIntervalInput = document.getElementById('backup-interval');
            // In dev mode, show the real Noita path in the input field
            if (noitaDirElement) {
                noitaDirElement.value = (this._isDevBuild && this._devSaveDir) ? defaultNoitaDir : effectiveNoitaDir;
            }
            if (entangledDirElement) entangledDirElement.value = defaultEntangledDir;
            if (darkModeElement) darkModeElement.checked = false;
            if (logLevelSelect) logLevelSelect.value = 'INFO';
            if (autoDeleteDaysInput) autoDeleteDaysInput.value = 30;
            if (backupIntervalInput) backupIntervalInput.value = 0;
            const monitorIntervalInput = document.getElementById('monitor-interval');
            const monitorMaxSnapshotsInput = document.getElementById('monitor-max-snapshots');
            if (monitorIntervalInput) monitorIntervalInput.value = 15;
            if (monitorMaxSnapshotsInput) monitorMaxSnapshotsInput.value = 10;
            this.uiManager.applyDarkMode();

            if (effectiveNoitaDir) {
                await this.modManager.loadModConfigFromDirectory(effectiveNoitaDir);
            }
            this.logAction('INFO', 'Settings reset to defaults. Press Save & Close to apply.');
        } catch (error) {
            this.logAction('ERROR', `Error resetting defaults: ${error.message}`);
        }
    }

    storeCurrentSettings() {
        this.previousSettings = { ...this.settings };
        this.previousIsDarkMode = state.isDarkMode;
        this._previousRealNoitaDir = this._realNoitaDir;
    }

    restorePreviousSettings() {
        if (this.previousSettings) {
            this.settings = { ...this.previousSettings };
            state.isDarkMode = this.previousIsDarkMode;
            this._realNoitaDir = this._previousRealNoitaDir;
            const noitaDirElement = document.getElementById('noita-dir');
            const entangledDirElement = document.getElementById('entangled-dir');
            const darkModeElement = document.getElementById('dark-mode-checkbox');
            const logLevelSelect = document.getElementById('log-level-select');
            const autoDeleteDaysInput = document.getElementById('auto-delete-days');
            const backupIntervalInput = document.getElementById('backup-interval');
            const monitorIntervalInput = document.getElementById('monitor-interval');
            const monitorMaxSnapshotsInput = document.getElementById('monitor-max-snapshots');
            // In dev mode, show the real Noita path in the input field
            if (noitaDirElement) {
                noitaDirElement.value = (this._isDevBuild && this._realNoitaDir) ? this._realNoitaDir : this.settings.noita_dir;
            }
            if (entangledDirElement) entangledDirElement.value = this.settings.entangled_dir;
            if (darkModeElement) darkModeElement.checked = state.isDarkMode;
            if (logLevelSelect) logLevelSelect.value = this.settings.log_settings.log_level;
            if (autoDeleteDaysInput) autoDeleteDaysInput.value = this.settings.backup_settings?.auto_delete_days ?? 30;
            if (backupIntervalInput) backupIntervalInput.value = this.settings.backup_settings?.backup_interval_minutes ?? 0;
            if (monitorIntervalInput) monitorIntervalInput.value = this.settings.save_monitor_settings?.interval_minutes ?? 15;
            if (monitorMaxSnapshotsInput) monitorMaxSnapshotsInput.value = this.settings.save_monitor_settings?.max_snapshots_per_preset ?? 10;
            this.uiManager.applyDarkMode();

            this.logAction('DEBUG', 'Restored Previous Settings');
        }
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'SettingsManager');
    }
}
