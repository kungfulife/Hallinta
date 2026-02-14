import { state } from './state.js';

export class SaveMonitorManager {
    constructor(uiManager, settingsManager) {
        this.uiManager = uiManager;
        this.settingsManager = settingsManager;
        this._monitorInterval = null;
        this._isRunning = false;
        this._includeEntangled = false;
    }

    get isRunning() {
        return this._isRunning;
    }

    async start(askEntangled = true) {
        if (this._isRunning) {
            this.logAction('WARN', 'Save Monitor is already running');
            return;
        }

        const settings = this.settingsManager.settings.save_monitor_settings || {};
        const intervalMinutes = settings.interval_minutes || 15;

        const noitaDir = this.settingsManager._isDevBuild && this.settingsManager._realNoitaDir
            ? this.settingsManager._realNoitaDir
            : this.settingsManager.settings.noita_dir;

        if (!noitaDir) {
            this.logAction('ERROR', 'Cannot start Save Monitor: Noita directory not set');
            return;
        }

        // Check if Entangled Worlds directory is configured and ask about inclusion
        const entangledDir = this.settingsManager.settings.entangled_dir || '';
        if (askEntangled && entangledDir) {
            this._includeEntangled = await new Promise((resolve) => {
                this.uiManager.showConfirmModal(
                    'Entangled Worlds directory detected. Include Entangled Worlds data in Save Monitor snapshots?', {
                        confirmText: 'Include',
                        cancelText: 'Skip',
                        onConfirm: () => resolve(true),
                        onCancel: () => resolve(false)
                    }
                );
            });
        } else {
            this._includeEntangled = settings.include_entangled || false;
        }

        this._isRunning = true;
        this._updateUI();
        this.logAction('INFO', `Save Monitor started (every ${intervalMinutes} min)`);

        // Take an initial snapshot
        await this._takeSnapshot();

        // Set up interval
        this._monitorInterval = setInterval(async () => {
            await this._takeSnapshot();
        }, intervalMinutes * 60 * 1000);
    }

    stop() {
        if (!this._isRunning) return;

        if (this._monitorInterval) {
            clearInterval(this._monitorInterval);
            this._monitorInterval = null;
        }

        this._isRunning = false;
        this._updateUI();
        this.logAction('INFO', 'Save Monitor stopped');
    }

    async _takeSnapshot() {
        const noitaDir = this.settingsManager._isDevBuild && this.settingsManager._realNoitaDir
            ? this.settingsManager._realNoitaDir
            : this.settingsManager.settings.noita_dir;

        if (!noitaDir) return;

        const presetName = state.selectedPreset || 'Default';
        const entangledDir = this.settingsManager.settings.entangled_dir || '';
        const maxSnapshots = this.settingsManager.settings.save_monitor_settings?.max_snapshots_per_preset || 10;

        try {
            const filename = await window.__TAURI__.core.invoke('create_monitor_snapshot', {
                noitaDir,
                presetName,
                includeEntangled: this._includeEntangled,
                entangledDir
            });
            this.logAction('INFO', `Save Monitor snapshot: ${filename} [${presetName}]`);

            // Cleanup old snapshots
            const deleted = await window.__TAURI__.core.invoke('cleanup_monitor_snapshots', {
                presetName,
                keepCount: maxSnapshots
            });
            if (deleted > 0) {
                this.logAction('DEBUG', `Cleaned up ${deleted} old snapshot(s) for preset "${presetName}"`);
            }
        } catch (error) {
            this.logAction('ERROR', `Save Monitor snapshot failed: ${error}`);
        }
    }

    _updateUI() {
        const btn = document.getElementById('save-monitor-toggle');
        if (btn) {
            btn.textContent = this._isRunning ? 'Stop Save Monitor' : 'Start Save Monitor';
            btn.className = this._isRunning
                ? 'save-monitor-btn save-monitor-active'
                : 'save-monitor-btn';
        }
        const statusIndicator = document.getElementById('save-monitor-status');
        if (statusIndicator) {
            statusIndicator.textContent = this._isRunning ? 'Running' : 'Stopped';
            statusIndicator.className = this._isRunning
                ? 'save-monitor-status active'
                : 'save-monitor-status';
        }
    }

    async confirmStopOnClose() {
        if (!this._isRunning) return true;

        return new Promise((resolve) => {
            this.uiManager.showConfirmModal(
                'Save Monitor is still running. Stop it and close the application?', {
                    confirmText: 'Stop & Close',
                    cancelText: 'Cancel',
                    onConfirm: () => {
                        this.stop();
                        resolve(true);
                    },
                    onCancel: () => resolve(false),
                    isImportant: true
                }
            );
        });
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'SaveMonitor');
    }
}
