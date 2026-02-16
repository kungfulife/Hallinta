import { state } from './state.js';
import { getCatalogUrl } from './catalogConfig.js';

export class GalleryManager {
    constructor(uiManager, settingsManager, presetManager) {
        this.uiManager = uiManager;
        this.settingsManager = settingsManager;
        this.presetManager = presetManager;
        this._catalogCache = null;
        this._catalogCacheTime = 0;
        this._activeTagFilters = new Set();
    }

    isGalleryOpen() {
        return state.galleryView;
    }

    async openGallery() {
        state.galleryView = true;
        this.uiManager.changeView('gallery');
        this._renderLoadingState();
        try {
            const catalog = await this.fetchCatalog(false);
            this._renderGallery(catalog);
        } catch (error) {
            this._renderErrorState(error.message || error);
        }
    }

    closeGallery() {
        state.galleryView = false;
        this._activeTagFilters.clear();
        this.uiManager.changeView('main');
    }

    async fetchCatalog(force) {
        const CACHE_DURATION_MS = 5 * 60 * 1000; // 5 minutes
        if (!force && this._catalogCache && (Date.now() - this._catalogCacheTime < CACHE_DURATION_MS)) {
            return this._catalogCache;
        }

        const catalogUrl = getCatalogUrl(this.settingsManager.settings.gallery_settings?.catalog_url || '');
        if (!catalogUrl) {
            throw new Error('Preset catalog URL is not configured by the developer build.');
        }

        const catalog = await window.__TAURI__.core.invoke('fetch_catalog', { catalogUrl });
        this._catalogCache = catalog;
        this._catalogCacheTime = Date.now();
        return catalog;
    }

    async refreshGallery() {
        this._renderLoadingState();
        try {
            const catalog = await this.fetchCatalog(true);
            this._renderGallery(catalog);
            this.logAction('INFO', 'Gallery refreshed');
        } catch (error) {
            this._renderErrorState(error.message || error);
        }
    }

    async downloadPreset(entry) {
        const downloadBtn = document.querySelector(`[data-preset-id="${entry.id}"] .gallery-card-download`);
        if (downloadBtn) {
            downloadBtn.classList.add('downloading');
            downloadBtn.textContent = 'Downloading...';
            downloadBtn.disabled = true;
        }

        try {
            const rawContent = await window.__TAURI__.core.invoke('download_preset_file', {
                downloadUrl: entry.download_url
            });

            let importData;
            try {
                importData = JSON.parse(rawContent);
            } catch (e) {
                throw new Error('Downloaded file is not valid JSON');
            }

            if (importData.hallinta_export !== 'presets' || !importData.presets) {
                throw new Error('Downloaded file is not a valid Hallinta preset export');
            }

            // Checksum verification
            if (entry.checksum) {
                try {
                    const presetsString = JSON.stringify(importData.presets);
                    const valid = await window.__TAURI__.core.invoke('verify_checksum', {
                        content: presetsString,
                        expectedChecksum: entry.checksum
                    });
                    if (!valid) {
                        const proceed = await new Promise((resolve) => {
                            this.uiManager.showConfirmModal(
                                'Checksum mismatch: the downloaded preset may have been modified since it was published. Continue importing?',
                                {
                                    confirmText: 'Continue',
                                    cancelText: 'Cancel',
                                    onConfirm: () => resolve(true),
                                    onCancel: () => resolve(false)
                                }
                            );
                        });
                        if (!proceed) {
                            this.logAction('INFO', 'Import cancelled due to checksum mismatch');
                            return;
                        }
                    }
                } catch (checksumError) {
                    this.logAction('WARN', `Checksum verification failed: ${checksumError}`);
                }
            }

            // Workshop mod check
            await this.checkWorkshopMods(importData);

            // Import the presets
            await this._importPresetData(importData);
            this.logAction('INFO', `Downloaded and imported preset: ${entry.name}`);
        } catch (error) {
            this.logAction('ERROR', `Failed to download preset: ${error.message || error}`);
        } finally {
            if (downloadBtn) {
                downloadBtn.classList.remove('downloading');
                downloadBtn.textContent = 'Download';
                downloadBtn.disabled = false;
            }
        }
    }

    async downloadByShareLink() {
        this.uiManager.showInputModal(
            'Enter a Google Drive share link to a Hallinta preset file:',
            '',
            async (url) => {
                if (!url || !url.trim()) {
                    this.logAction('INFO', 'No URL provided');
                    return;
                }

                try {
                    const downloadUrl = await window.__TAURI__.core.invoke('parse_gdrive_share_link', { url: url.trim() });
                    const rawContent = await window.__TAURI__.core.invoke('download_preset_file', { downloadUrl });

                    let importData;
                    try {
                        importData = JSON.parse(rawContent);
                    } catch (e) {
                        throw new Error('Downloaded file is not valid JSON');
                    }

                    if (importData.hallinta_export !== 'presets' || !importData.presets) {
                        throw new Error('Downloaded file is not a valid Hallinta preset export');
                    }

                    // Checksum verification if present
                    if (importData.checksum) {
                        try {
                            const presetsString = JSON.stringify(importData.presets);
                            const valid = await window.__TAURI__.core.invoke('verify_checksum', {
                                content: presetsString,
                                expectedChecksum: importData.checksum
                            });
                            if (!valid) {
                                const proceed = await new Promise((resolve) => {
                                    this.uiManager.showConfirmModal(
                                        'Checksum mismatch: the downloaded preset may have been modified. Continue importing?',
                                        {
                                            confirmText: 'Continue',
                                            cancelText: 'Cancel',
                                            onConfirm: () => resolve(true),
                                            onCancel: () => resolve(false)
                                        }
                                    );
                                });
                                if (!proceed) {
                                    this.logAction('INFO', 'Import cancelled due to checksum mismatch');
                                    return;
                                }
                            }
                        } catch (checksumError) {
                            this.logAction('WARN', `Checksum verification failed: ${checksumError}`);
                        }
                    }

                    await this.checkWorkshopMods(importData);
                    await this._importPresetData(importData);
                    this.logAction('INFO', 'Imported preset from share link');
                } catch (error) {
                    this.logAction('ERROR', `Failed to import from link: ${error.message || error}`);
                }
            },
            () => {
                this.logAction('INFO', 'Share link import cancelled');
            }
        );
    }

    async checkWorkshopMods(presetData) {
        // Collect all unique workshop IDs from the preset data
        const workshopIds = new Set();
        const modNamesByWorkshopId = {};

        for (const presetName of Object.keys(presetData.presets)) {
            for (const mod of presetData.presets[presetName]) {
                const id = mod.workshop_id || '0';
                if (id !== '0' && id !== '') {
                    workshopIds.add(id);
                    if (!modNamesByWorkshopId[id]) {
                        modNamesByWorkshopId[id] = mod.name;
                    }
                }
            }
        }

        if (workshopIds.size === 0) return;

        let steamPath = this.settingsManager.settings.gallery_settings?.steam_path || '';
        if (!steamPath) {
            try {
                steamPath = await window.__TAURI__.core.invoke('detect_steam_path');
            } catch (e) {
                this.logAction('WARN', 'Steam not found. Skipping workshop mod check.');
                return;
            }
        }

        try {
            const statuses = await window.__TAURI__.core.invoke('check_workshop_mods_installed', {
                workshopIds: Array.from(workshopIds),
                steamPath
            });

            const missing = statuses.filter(s => !s.installed);
            if (missing.length === 0) return;

            // Show missing mods modal
            await new Promise((resolve) => {
                this._showMissingModsModal(missing, modNamesByWorkshopId, resolve);
            });
        } catch (error) {
            this.logAction('WARN', `Workshop mod check failed: ${error}`);
        }
    }

    _showMissingModsModal(missingMods, modNamesByWorkshopId, onDone) {
        if (state.isModalVisible) {
            onDone();
            return;
        }
        state.isModalVisible = true;

        const modal = document.createElement('div');
        modal.className = 'custom-modal';

        const rows = missingMods.map(mod => {
            const name = window.logUtils?.escapeHtml(modNamesByWorkshopId[mod.workshop_id] || mod.workshop_id) || mod.workshop_id;
            return `<div class="missing-mod-row">
                <span class="missing-mod-name">${name}</span>
                <span class="missing-mod-id">ID: ${window.logUtils?.escapeHtml(mod.workshop_id) || mod.workshop_id}</span>
                <button class="missing-mod-subscribe" data-workshop-id="${mod.workshop_id}">Subscribe</button>
            </div>`;
        }).join('');

        modal.innerHTML = `
            <div class="modal-content-checklist">
                <h3>Missing Workshop Mods</h3>
                <p>${missingMods.length} mod(s) from this preset are not installed. You can subscribe to them on Steam before continuing.</p>
                <div class="missing-mods-list themed-scrollbar-compact">${rows}</div>
                <div class="modal-buttons">
                    <button id="modal-confirm">Continue Import</button>
                    <button id="modal-cancel">Cancel</button>
                </div>
            </div>
        `;

        document.body.appendChild(modal);

        // Subscribe buttons
        modal.querySelectorAll('.missing-mod-subscribe').forEach(btn => {
            btn.addEventListener('click', async () => {
                const workshopId = btn.dataset.workshopId;
                try {
                    await window.__TAURI__.core.invoke('open_steam_subscribe', { workshopId });
                    btn.textContent = 'Opened';
                    btn.disabled = true;
                } catch (e) {
                    this.logAction('ERROR', `Failed to open Steam subscribe: ${e}`);
                }
            });
        });

        const closeModal = () => {
            if (modal.parentNode) document.body.removeChild(modal);
            document.removeEventListener('keydown', escapeHandler);
            state.isModalVisible = false;
        };

        const escapeHandler = (e) => {
            if (e.key === 'Escape') {
                this.logAction('DEBUG', 'Escape keybind triggered: close missing Workshop mods modal');
                closeModal();
                onDone();
            }
        };

        modal.querySelector('#modal-confirm').addEventListener('click', () => {
            closeModal();
            onDone();
        });

        modal.querySelector('#modal-cancel').addEventListener('click', () => {
            closeModal();
            onDone();
        });

        document.addEventListener('keydown', escapeHandler);
    }

    async _importPresetData(importData) {
        const importedNames = Object.keys(importData.presets);
        let imported = 0;

        for (const name of importedNames) {
            let targetName = name;
            if (state.currentPresets[name]) {
                // Rename to avoid overwrite by default
                targetName = `${name} (vault)`;
                let counter = 2;
                while (state.currentPresets[targetName]) {
                    targetName = `${name} (vault ${counter})`;
                    counter++;
                }
            }

            state.currentPresets[targetName] = importData.presets[name].map(mod => ({
                name: mod.name,
                enabled: mod.enabled,
                workshopId: mod.workshop_id || '0',
                settingsFoldOpen: mod.settings_fold_open || false,
                index: 0
            }));
            imported++;
        }

        await this.presetManager.saveSelectedPreset();
        this.presetManager.loadPresets();
        this.logAction('INFO', `Imported ${imported} preset(s)`);
    }

    // --- Rendering ---

    _renderGallery(catalog) {
        const content = document.getElementById('gallery-content');
        if (!content) return;

        if (!catalog.presets || catalog.presets.length === 0) {
            content.innerHTML = '<div class="gallery-empty">No presets available in the catalog.</div>';
            this._renderTags([]);
            return;
        }

        const allTags = this._getAllTags(catalog.presets);
        this._renderTags(allTags);
        this._filterAndRender(catalog.presets);
    }

    _renderLoadingState() {
        const content = document.getElementById('gallery-content');
        if (content) {
            content.innerHTML = '<div class="gallery-loading"><div class="gallery-spinner"></div><p>Loading catalog...</p></div>';
        }
    }

    _renderErrorState(message) {
        const content = document.getElementById('gallery-content');
        if (content) {
            const escaped = window.logUtils?.escapeHtml(message) || message;
            content.innerHTML = `<div class="gallery-error">
                <p>${escaped}</p>
                <button onclick="refreshGallery()">Retry</button>
            </div>`;
        }
    }

    filterAndRender() {
        if (!this._catalogCache) return;
        this._filterAndRender(this._catalogCache.presets || []);
    }

    _filterAndRender(presets) {
        const content = document.getElementById('gallery-content');
        if (!content) return;

        const searchInput = document.getElementById('gallery-search');
        const searchTerm = (searchInput?.value || '').toLowerCase();

        let filtered = presets;

        // Filter by search
        if (searchTerm) {
            filtered = filtered.filter(p =>
                p.name.toLowerCase().includes(searchTerm) ||
                p.description.toLowerCase().includes(searchTerm) ||
                p.author.toLowerCase().includes(searchTerm)
            );
        }

        // Filter by tags
        if (this._activeTagFilters.size > 0) {
            filtered = filtered.filter(p =>
                p.tags.some(tag => this._activeTagFilters.has(tag))
            );
        }

        if (filtered.length === 0) {
            content.innerHTML = '<div class="gallery-empty">No presets match your filters.</div>';
            return;
        }

        content.innerHTML = filtered.map(entry => this._renderCard(entry)).join('');

        // Attach download handlers
        content.querySelectorAll('.gallery-card-download').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.stopPropagation();
                const presetId = btn.closest('.gallery-card').dataset.presetId;
                const entry = filtered.find(p => p.id === presetId);
                if (entry) this.downloadPreset(entry);
            });
        });
    }

    _renderCard(entry) {
        const name = window.logUtils?.escapeHtml(entry.name) || entry.name;
        const author = window.logUtils?.escapeHtml(entry.author) || entry.author;
        const description = window.logUtils?.escapeHtml(entry.description) || entry.description;
        const tags = entry.tags.map(t =>
            `<span class="gallery-card-tag">${window.logUtils?.escapeHtml(t) || t}</span>`
        ).join('');

        return `<div class="gallery-card" data-preset-id="${entry.id}">
            <div class="gallery-card-header">
                <div class="gallery-card-title">${name}</div>
                <div class="gallery-card-author">by ${author}</div>
            </div>
            <div class="gallery-card-description">${description}</div>
            <div class="gallery-card-tags">${tags}</div>
            <div class="gallery-card-footer">
                <span class="gallery-card-mod-count">${entry.mod_count} mods</span>
                <span class="gallery-card-version">v${window.logUtils?.escapeHtml(entry.version) || entry.version}</span>
                <button class="gallery-card-download">Download</button>
            </div>
        </div>`;
    }

    _renderTags(allTags) {
        const container = document.getElementById('gallery-tags');
        if (!container) return;

        if (allTags.length === 0) {
            container.innerHTML = '';
            return;
        }

        container.innerHTML = allTags.map(tag => {
            const active = this._activeTagFilters.has(tag) ? ' active' : '';
            return `<button class="gallery-tag${active}" data-tag="${tag}">${window.logUtils?.escapeHtml(tag) || tag}</button>`;
        }).join('');

        container.querySelectorAll('.gallery-tag').forEach(btn => {
            btn.addEventListener('click', () => {
                const tag = btn.dataset.tag;
                if (this._activeTagFilters.has(tag)) {
                    this._activeTagFilters.delete(tag);
                    btn.classList.remove('active');
                } else {
                    this._activeTagFilters.add(tag);
                    btn.classList.add('active');
                }
                this.filterAndRender();
            });
        });
    }

    _getAllTags(presets) {
        const tagSet = new Set();
        for (const preset of presets) {
            for (const tag of preset.tags) {
                tagSet.add(tag);
            }
        }
        return Array.from(tagSet).sort();
    }

    logAction(level, message) {
        this.uiManager.logAction(level, message, 'GalleryManager');
    }
}
