import { state } from './state.js';

export class BackupManager {
    constructor(uiManager, modManager, settingsManager, presetManager) {
        this.uiManager = uiManager;
        this.modManager = modManager;
        this.settingsManager = settingsManager;
        this.presetManager = presetManager;
        this._autoBackupInterval = null;
    }

    async createBackup() {
        if (state.backupInProgress) {
            this.logAction('WARN', 'A backup is already in progress');
            return;
        }

        const noitaDir = this.settingsManager._isDevBuild && this.settingsManager._realNoitaDir
            ? this.settingsManager._realNoitaDir
            : this.settingsManager.settings.noita_dir;

        if (!noitaDir) {
            this.logAction('ERROR', 'Cannot create backup: Noita directory not set');
            return;
        }

        const checklistItems = [
            { id: 'save01', label: 'Include save01 (modded save run)', checked: false },
            { id: 'presets', label: 'Include presets', checked: true }
        ];

        // Offer Entangled Worlds inclusion if directory is configured
        const entangledDir = this.settingsManager.settings.entangled_dir || '';
        if (entangledDir) {
            checklistItems.push({ id: 'entangled', label: 'Include Entangled Worlds data', checked: false });
        }

        this.uiManager.showChecklistModal(
            'Create Backup',
            'Select what to include in the backup:',
            checklistItems,
            async (selected) => {
                state.backupInProgress = true;
                this.logAction('DEBUG', 'Starting backup creation...');
                this.uiManager.logAction('INFO', 'Creating backup... This may take a moment.');

                try {
                    const includeSave01 = selected.includes('save01');
                    const includePresets = selected.includes('presets');
                    const includeEntangled = selected.includes('entangled');

                    const filename = await window.__TAURI__.core.invoke('create_backup', {
                        noitaDir,
                        includeSave01,
                        includePresets,
                        includeEntangled,
                        entangledDir: includeEntangled ? entangledDir : ''
                    });

                    this.logAction('INFO', `Backup created successfully: ${filename}`);
                } catch (error) {
                    this.logAction('ERROR', `Failed to create backup: ${error}`);
                } finally {
                    state.backupInProgress = false;
                }
            },
            () => {
                this.logAction('INFO', 'Backup creation cancelled');
            }
        );
    }

    async listBackups() {
        try {
            return await window.__TAURI__.core.invoke('list_backups');
        } catch (error) {
            this.logAction('ERROR', `Failed to list backups: ${error}`);
            return [];
        }
    }

    async openRestoreUI() {
        this.logAction('DEBUG', 'Opening restore UI');
        try {
            const backups = await this.listBackups();

            if (backups.length === 0) {
                this.uiManager.logAction('INFO', 'No backups found');
                return;
            }

            // Build backup selection list
            const items = backups.map(b => {
                const date = new Date(b.timestamp).toLocaleString();
                const sizeMB = (b.size_bytes / (1024 * 1024)).toFixed(1);
                const contents = [];
                if (b.contains_save00) contents.push('save00');
                if (b.contains_save01) contents.push('save01');
                if (b.contains_presets) contents.push('presets');
                if (b.contains_entangled) contents.push('entangled');
                return {
                    id: b.filename,
                    label: `${date} (${sizeMB} MB) - ${contents.join(', ')}`,
                    checked: false,
                    backup: b
                };
            });

            // Show backup selection modal
            this._showBackupSelectionModal(items);
        } catch (error) {
            this.logAction('ERROR', `Failed to open restore UI: ${error}`);
        }
    }

    _showBackupSelectionModal(items) {
        if (state.isModalVisible) {
            this.logAction('WARN', 'A modal is already open');
            return;
        }
        state.isModalVisible = true;

        const modal = document.createElement('div');
        modal.className = 'custom-modal';

        let listHTML = items.map(item => {
            return `<div class="backup-item" data-filename="${item.id}">
                <span>${item.label}</span>
            </div>`;
        }).join('');

        modal.innerHTML = `
            <div class="modal-content-checklist backup-selection-modal">
                <h3>Select Backup to Restore</h3>
                <div class="backup-list-container">
                    ${listHTML}
                </div>
                <div class="modal-buttons">
                    <button id="modal-cancel">Cancel</button>
                </div>
            </div>
        `;

        document.body.appendChild(modal);

        const closeModal = () => {
            if (modal.parentNode) document.body.removeChild(modal);
            document.removeEventListener('keydown', escapeHandler);
            state.isModalVisible = false;
        };

        const escapeHandler = (e) => {
            if (e.key === 'Escape') closeModal();
        };

        document.addEventListener('keydown', escapeHandler);
        modal.querySelector('#modal-cancel').addEventListener('click', closeModal);

        // Click handler for backup items
        modal.querySelectorAll('.backup-item').forEach(el => {
            el.addEventListener('click', () => {
                const filename = el.dataset.filename;
                const backup = items.find(i => i.id === filename)?.backup;
                closeModal();
                if (backup) {
                    this._showRestoreOptionsModal(backup);
                }
            });

        });
    }

    _showRestoreOptionsModal(backup) {
        const checklistItems = [];
        if (backup.contains_save00) {
            checklistItems.push({ id: 'restore_save00', label: 'Restore save00', checked: true });
        }
        if (backup.contains_save01) {
            checklistItems.push({ id: 'restore_save01', label: 'Restore save01 (modded save)', checked: true });
        }
        if (backup.contains_presets) {
            checklistItems.push({ id: 'restore_presets', label: 'Restore presets', checked: true });
        }
        if (backup.contains_entangled) {
            checklistItems.push({ id: 'restore_entangled', label: 'Restore Entangled Worlds data', checked: true });
        }

        if (checklistItems.length === 0) {
            this.logAction('ERROR', 'Backup appears to be empty');
            return;
        }

        this.uiManager.showChecklistModal(
            'Restore Options',
            'Close Noita before restoring. Restoring from: ' + new Date(backup.timestamp).toLocaleString(),
            checklistItems,
            async (selected) => {
                await this._performRestore(backup.filename, {
                    restore_save00: selected.includes('restore_save00'),
                    restore_save01: selected.includes('restore_save01'),
                    restore_presets: selected.includes('restore_presets'),
                    restore_entangled: selected.includes('restore_entangled')
                }, selected.includes('restore_presets'));
            },
            () => {
                this.logAction('INFO', 'Restore cancelled');
            }
        );
    }

    async _performRestore(filename, options, presetsRestored) {
        state.isRestoring = true;
        this.logAction('DEBUG', `Starting restore from: ${filename}`);
        this.uiManager.logAction('INFO', 'Restoring backup... Please wait.');

        try {
            const noitaDir = this.settingsManager._isDevBuild && this.settingsManager._realNoitaDir
                ? this.settingsManager._realNoitaDir
                : this.settingsManager.settings.noita_dir;

            const entangledDir = this.settingsManager.settings.entangled_dir || '';
            await window.__TAURI__.core.invoke('restore_backup', {
                filename,
                noitaDir,
                options,
                entangledDir: options.restore_entangled ? entangledDir : ''
            });

            this.logAction('INFO', `Backup restored successfully from: ${filename}`);

            if (presetsRestored) {
                // Reload presets from disk
                try {
                    const presets = await window.__TAURI__.core.invoke('load_presets');
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

                    if (!state.currentPresets[state.selectedPreset]) {
                        state.selectedPreset = 'Default';
                    }

                    this.presetManager.loadPresets();
                    await this.presetManager.loadToSelectedPreset();
                    this.logAction('INFO', 'Presets reloaded from restored backup');
                } catch (e) {
                    this.logAction('ERROR', `Failed to reload presets after restore: ${e}`);
                }
            }
        } catch (error) {
            this.logAction('ERROR', `Failed to restore backup: ${error}`);
        } finally {
            state.isRestoring = false;
        }
    }

    startAutoBackup(intervalMinutes) {
        this.stopAutoBackup();
        if (!intervalMinutes || intervalMinutes <= 0) return;

        this.logAction('DEBUG', `Starting auto-backup every ${intervalMinutes} minutes`);
        this._autoBackupInterval = setInterval(async () => {
            if (state.backupInProgress || state.isRestoring) return;

            const noitaDir = this.settingsManager._isDevBuild && this.settingsManager._realNoitaDir
                ? this.settingsManager._realNoitaDir
                : this.settingsManager.settings.noita_dir;

            if (!noitaDir) return;

            state.backupInProgress = true;
            try {
                const filename = await window.__TAURI__.core.invoke('create_backup', {
                    noitaDir,
                    includeSave01: false,
                    includePresets: true
                });
                this.logAction('INFO', `Auto-backup created: ${filename}`);
            } catch (error) {
                this.logAction('ERROR', `Auto-backup failed: ${error}`);
            } finally {
                state.backupInProgress = false;
            }
        }, intervalMinutes * 60 * 1000);
    }

    stopAutoBackup() {
        if (this._autoBackupInterval) {
            clearInterval(this._autoBackupInterval);
            this._autoBackupInterval = null;
        }
    }

    async cleanupOldBackups() {
        const maxDays = this.settingsManager.settings.backup_settings?.auto_delete_days || 30;
        if (maxDays === 0) return;

        try {
            const deleted = await window.__TAURI__.core.invoke('cleanup_old_backups', { maxAgeDays: maxDays });
            if (deleted > 0) {
                this.logAction('INFO', `Cleaned up ${deleted} old backup(s)`);
            }
        } catch (error) {
            this.logAction('ERROR', `Failed to cleanup old backups: ${error}`);
        }
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'BackupManager');
    }
}
