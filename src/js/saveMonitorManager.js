import { state } from './state.js';

export class SaveMonitorManager {
    constructor(uiManager, settingsManager) {
        this.uiManager = uiManager;
        this.settingsManager = settingsManager;
        this._monitorInterval = null;
        this._isRunning = false;
        this._includeEntangled = false;
        this._lastBlockedNoticeAt = 0;
    }

    get isRunning() {
        return this._isRunning;
    }

    isInteractionBlocked(actionLabel = 'This action') {
        if (!this._isRunning) return false;
        const now = Date.now();
        if (now - this._lastBlockedNoticeAt > 1200) {
            this._lastBlockedNoticeAt = now;
            this.uiManager.logAction(
                'INFO',
                `${actionLabel} is disabled while Save Monitor is running.`,
                'SaveMonitor'
            );
        }
        return true;
    }

    async start(askEntangled = true, options = {}) {
        if (this._isRunning) {
            this.logAction('WARN', 'Save Monitor is already running');
            return;
        }

        const settings = this.settingsManager.settings.save_monitor_settings || {};
        const intervalMinutes = settings.interval_minutes || 3;
        const startupLaunch = !!options.startup;

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
        this.logAction(
            'INFO',
            startupLaunch
                ? `Save Monitor started on launch (every ${intervalMinutes} min)`
                : `Save Monitor started (every ${intervalMinutes} min)`
        );

        // Disable sortable while monitor is running
        const sortable = window.__hallintaSortable;
        if (sortable && typeof sortable.option === 'function') {
            sortable.option('disabled', true);
        }

        // Take an initial snapshot
        await this._takeSnapshot();

        // Set up interval
        this._monitorInterval = setInterval(async () => {
            await this._takeSnapshot();
        }, intervalMinutes * 60 * 1000);
    }

    async stop() {
        if (!this._isRunning) return;

        if (this._monitorInterval) {
            clearInterval(this._monitorInterval);
            this._monitorInterval = null;
        }

        this._isRunning = false;
        this._updateUI();
        this.logAction('INFO', 'Save Monitor stopped');

        // Re-enable sortable if not in compact mode
        const sortable = window.__hallintaSortable;
        if (sortable && typeof sortable.option === 'function') {
            sortable.option('disabled', !!state.compactMode);
        }
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

            // Update snapshot info in monitor panel
            const snapshotInfo = document.getElementById('monitor-snapshot-info');
            if (snapshotInfo) {
                const now = new Date();
                snapshotInfo.textContent = `Last snapshot: ${now.toLocaleTimeString()}`;
            }

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

    async takeExitSnapshot() {
        this.logAction('INFO', 'Taking exit snapshot before closing');
        await this._takeSnapshot();
        this.logAction('INFO', 'Exit snapshot complete');
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

        // Update monitor panel info
        const presetNameEl = document.getElementById('monitor-preset-name');
        if (presetNameEl) {
            presetNameEl.textContent = state.selectedPreset || 'Default';
        }
        const snapshotInfo = document.getElementById('monitor-snapshot-info');
        if (snapshotInfo && !this._isRunning) {
            snapshotInfo.textContent = '';
        }
    }

    async confirmStopOnClose() {
        if (!this._isRunning) return true;

        return new Promise((resolve) => {
            this.uiManager.showConfirmModal(
                'Save Monitor is running. Take a final snapshot and close?', {
                    confirmText: 'Snapshot & Close',
                    cancelText: 'Cancel',
                    onConfirm: () => resolve(true),
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
