import {state} from './state.js';

export class UIManager {

    constructor() {
        // Set in main.js
        this.modManager = null;
        this.settingsManager = null;
    }

    changeView(view) {
        this.logAction('DEBUG', `Changing view to ${view}`);
        const mainPage = document.getElementById('main-page');
        const settingsPage = document.getElementById('settings-page');
        const presetControls = document.getElementById('preset-controls');
        const combinedButton = document.getElementById('header-combined-button');
        const searchBar = document.getElementById('search-bar');
        const TRANSITION_DURATION_MS = 200;

        function cleanupAnimation(el) {
            el.classList.remove('fade-in-fast', 'fade-out-fast');
        }

        if (view === 'main') {
            cleanupAnimation(presetControls);
            cleanupAnimation(searchBar);

            presetControls.style.display = 'flex';
            searchBar.style.display = 'flex';

            void presetControls.offsetWidth;
            void searchBar.offsetWidth;

            presetControls.classList.add('fade-in-fast');
            searchBar.classList.add('fade-in-fast');

            mainPage.style.display = 'flex';
            settingsPage.style.display = 'none';
            combinedButton.textContent = 'Settings';
            combinedButton.className = 'header-combined-button settings-state';
        } else if (view === 'settings') {
            cleanupAnimation(presetControls);
            cleanupAnimation(searchBar);

            presetControls.classList.add('fade-out-fast');
            searchBar.classList.add('fade-out-fast');

            setTimeout(() => {
                presetControls.style.display = 'none';
                searchBar.style.display = 'none';
                cleanupAnimation(presetControls);
                cleanupAnimation(searchBar);
            }, TRANSITION_DURATION_MS);
            mainPage.style.display = 'none';
            settingsPage.style.display = 'block';

            combinedButton.textContent = 'Cancel';
            combinedButton.className = 'header-combined-button cancel-state';
        }
    }

    renderModList() {
        this.logAction('DEBUG', 'Rendering mod list');
        const modList = document.getElementById('mod-list');
        modList.innerHTML = '';

        state.currentMods.forEach((mod, index) => {
            const li = document.createElement('li');
            li.className = 'mod-item';
            li.setAttribute('data-index', index);

            li.addEventListener('click', (e) => {
                if (e.button !== 2) {
                    e.stopPropagation();
                    this.modManager.toggleMod(index);
                    this.renderModList();
                }
            });

            if (mod.enabled) {
                li.classList.add('mod-enabled');
            } else {
                li.classList.add('mod-disabled');
            }

            const numberDiv = document.createElement('div');
            numberDiv.className = 'mod-number';
            numberDiv.textContent = index + 1;

            const infoDiv = document.createElement('div');
            infoDiv.className = 'mod-info';

            const nameSpan = document.createElement('span');
            nameSpan.className = 'mod-name';
            nameSpan.textContent = mod.name;

            const typeSpan = document.createElement('span');
            typeSpan.className = 'mod-type';
            typeSpan.textContent = mod.workshopId !== '0' ? `Workshop ID: ${mod.workshopId}` : 'Local Mod';

            if (mod.workshopId !== '0') {
                typeSpan.classList.add('workshop');
            } else {
                typeSpan.classList.add('local');
            }

            infoDiv.appendChild(nameSpan);
            infoDiv.appendChild(typeSpan);

            const checkboxContainer = document.createElement('div');
            checkboxContainer.className = 'checkbox-container';

            const checkbox = document.createElement('div');
            checkbox.className = mod.enabled ? 'visual-checkbox checked' : 'visual-checkbox';
            checkbox.innerHTML = mod.enabled ? '✓' : '';

            checkboxContainer.appendChild(checkbox);

            li.appendChild(numberDiv);
            li.appendChild(infoDiv);
            li.appendChild(checkboxContainer);

            modList.appendChild(li);
        });

        // Reapply the search filter to preserve the current search state
        this.filterMods();
    }

    updateModCount() {
        const modCount = document.getElementById('mod-count');
        const enabledCount = state.currentMods.filter(mod => mod.enabled).length;
        modCount.textContent = `Total Mods: ${state.currentMods.length} (${enabledCount} enabled)`;
    }

    filterMods() {
        this.logAction('DEBUG', 'Filtering mods');
        const searchTerm = document.querySelector('.search-bar').value.toLowerCase();
        const modItems = document.querySelectorAll('.mod-item');

        modItems.forEach(item => {
            const modName = item.querySelector('.mod-name').textContent.toLowerCase();
            item.style.display = modName.includes(searchTerm) ? 'flex' : 'none';
        });
    }

    toggleDarkMode() {
        this.logAction('DEBUG', `Toggled dark mode to ${state.isDarkMode}`);
        const checkbox = document.getElementById('dark-mode-checkbox');
        state.isDarkMode = checkbox.checked;
        this.applyDarkMode();
    }

    applyDarkMode() {
        document.body.classList.toggle('dark-mode', state.isDarkMode);
    }

    // ShowConfirmModal allows you to display a customizable two option request to the user.
    // by default "isImportant" is false, allowing for ease ESC Key/Clicking outside of request to used to cancel.
    showConfirmModal(message, options = {}) {
        // this.logAction('DEBUG', `Showing Confirmation modal - ${message}`);
        const {
            confirmText = 'Confirm',
            cancelText = 'Cancel',
            onConfirm = () => {
            },
            onCancel = () => {
            },
            isImportant = false
        } = options;

        if (state.isModalVisible) {
            this.logAction('WARN', 'A modal already open. New request ignored.');
            return;
        }
        state.isModalVisible = true;

        const modal = document.createElement('div');
        modal.className = 'custom-modal';
        modal.innerHTML = `
            <div class="modal-content">
                <p>${message}</p>
                <div class="modal-buttons">
                    <button id="modal-confirm">${confirmText}</button>
                    <button id="modal-cancel">${cancelText}</button>
                </div>
            </div>
        `;

        document.body.appendChild(modal);

        const confirmButton = document.getElementById('modal-confirm');
        const cancelButton = document.getElementById('modal-cancel');

        const closeModal = () => {
            if (modal.parentNode) {
                document.body.removeChild(modal);
            }

            document.removeEventListener('keydown', escapeHandler);

            state.isModalVisible = false;
        };

        const confirmAction = () => {
            onConfirm();
            closeModal();
        };

        const cancelAction = () => {
            onCancel();
            closeModal();
        };

        const escapeHandler = (e) => {
            if (e.key === 'Escape') {
                cancelAction();
            }
        };

        confirmButton.addEventListener('click', confirmAction);
        cancelButton.addEventListener('click', cancelAction);

        // Allows ESC and Click, canceling shortcuts.
        document.addEventListener('keydown', escapeHandler);
        if (!isImportant)
            modal.addEventListener('click', (e) => {
                if (e.target === modal) {
                    cancelAction();
                }
            });
    }

    showInputModal(message, defaultValue, onConfirm, onCancel) {
        if (state.isModalVisible) {
            this.logAction('WARN', 'A modal is already open. New request ignored.');
            return;
        }
        state.isModalVisible = true;

        const modal = document.createElement('div');
        modal.className = 'custom-modal';
        modal.innerHTML = `
            <div class="modal-content">
                <p>${message}</p>
                <input type="text" id="modal-input" value="${defaultValue}">
                <div class="modal-buttons">
                    <button id="modal-confirm">OK</button>
                    <button id="modal-cancel">Cancel</button>
                </div>
            </div>
        `;
        document.body.appendChild(modal);

        const confirmButton = document.getElementById('modal-confirm');
        const cancelButton = document.getElementById('modal-cancel');

        const closeModal = () => {
            if (modal.parentNode) {
                document.body.removeChild(modal);
            }

            document.removeEventListener('keydown', escapeHandler);

            state.isModalVisible = false;
        };

        const confirmAction = () => {
            const input = document.getElementById('modal-input').value;
            onConfirm(input);
            closeModal();
        };

        const cancelAction = () => {
            onCancel();
            closeModal();
        };

        const escapeHandler = (e) => {
            if (e.key === 'Escape') {
                cancelAction();
            }
        };

        confirmButton.addEventListener('click', confirmAction);
        cancelButton.addEventListener('click', cancelAction);

        // Allows ESC and Click, canceling shortcuts.
        document.addEventListener('keydown', escapeHandler);
    }

    showMissingModsModal(missingMods, onContinue) {
        const modNames = missingMods.map(m => m.name).join(', ');
        this.showConfirmModal(
            `${missingMods.length} mod(s) from the import were not found in your mod list and will be skipped: ${modNames}`, {
                confirmText: 'Continue Import',
                cancelText: 'Cancel Import',
                onConfirm: onContinue,
                onCancel: () => {
                    this.logAction('INFO', 'Import cancelled due to missing mods.');
                }
            }
        );
    }

    toggleMod() {
        if (state.contextMenuTarget !== null) {
            this.modManager.toggleMod(state.contextMenuTarget);
            this.renderModList();
            document.getElementById('mod-context-menu').style.display = 'none';
        }
    }

    deleteMod() {
        if (state.contextMenuTarget !== null) {
            const modName = state.currentMods[state.contextMenuTarget].name;
            this.showConfirmModal(
                `Are you sure you want to delete the mod "${modName}"?`, {
                    confirmText: 'Delete',
                    cancelText: 'Cancel',
                    onConfirm: () => {
                        this.modManager.deleteMod(state.contextMenuTarget);
                        this.logAction('DEBUG', `Deleted mod: ${modName}`);
                        document.getElementById('mod-context-menu').style.display = 'none';
                    },
                    onCancel: () => {
                        this.logAction('INFO', `Deletion of "${modName}" canceled`);
                        document.getElementById('mod-context-menu').style.display = 'none';
                    }
                }
            );
        }
    }

    reorderMod() {
        if (state.contextMenuTarget !== null) {
            const modName = state.currentMods[state.contextMenuTarget].name;
            this.showInputModal(
                `Enter new position for "${modName}" (1-${state.currentMods.length}):`,
                state.contextMenuTarget + 1,
                async (input) => {
                    const newIndex = parseInt(input) - 1;
                    if (isNaN(newIndex) || newIndex < 0 || newIndex >= state.currentMods.length) {
                        this.logAction('ERROR', `Invalid position for "${modName}"`);
                    } else {
                        this.modManager.reorderMod(state.contextMenuTarget, newIndex);
                        await this.modManager.finishReordering();
                        this.logAction('DEBUG', `Moved "${modName}" to position ${newIndex + 1}`);
                    }
                    document.getElementById('mod-context-menu').style.display = 'none';
                },
                () => {
                    this.logAction('INFO', `Reordering of "${modName}" canceled`);
                    document.getElementById('mod-context-menu').style.display = 'none';
                }
            );
        }
    }

    async openWorkshop() {
        if (state.contextMenuTarget !== null) {
            const mod = state.currentMods[state.contextMenuTarget];
            if (mod.workshopId !== '0' && window.__TAURI__) {
                try {
                    await window.__TAURI__.core.invoke('open_workshop_item', {workshopId: mod.workshopId});
                    this.logAction('DEBUG', `Opened workshop page for ${mod.name}`);
                } catch (error) {
                    this.logAction('ERROR', `Error opening workshop: ${error.message}`);
                    const url = `https://steamcommunity.com/sharedfiles/filedetails/?id=${mod.workshopId}`;
                    window.open(url, '_blank');
                    this.logAction('DEBUG', `Opened workshop page for ${mod.name}`);
                }
            }
            document.getElementById('mod-context-menu').style.display = 'none';
        }
    }

    async copyWorkshopLink() {
        if (state.contextMenuTarget !== null) {
            const mod = state.currentMods[state.contextMenuTarget];
            const url = `https://steamcommunity.com/sharedfiles/filedetails/?id=${mod.workshopId}`;
            try {
                await navigator.clipboard.writeText(url);
                this.logAction('DEBUG', `Copied workshop link for ${mod.name}`);
            } catch (error) {
                this.logAction('ERROR', `Error copying to clipboard: ${error.message}`);
            }
            document.getElementById('mod-context-menu').style.display = 'none';
        }
    }

    logAction(level, message, module = 'UIManager') {
        const normalizedLevel = level.toUpperCase(); // Normalize to uppercase
        const logLevelOrder = {'DEBUG': 0, 'INFO': 1, 'WARN': 2, 'ERROR': 3};
        const currentLogLevel = this.settingsManager?.settings.log_settings.log_level?.toUpperCase() || 'INFO';
        const currentLevelValue = logLevelOrder[currentLogLevel] ?? 1;
        const logLevelValue = logLevelOrder[normalizedLevel] ?? 0;

        if (logLevelValue >= currentLevelValue) {

            // Print to browser console.
            // console.log(`Logging: [${normalizedLevel}] ${message} (Current Level: ${currentLogLevel}, Log Value: ${logLevelValue}, Current Value: ${currentLevelValue})`);

            const statusBar = document.getElementById('status-bar');
            if (statusBar) {
                if (normalizedLevel === 'ERROR') {
                    statusBar.textContent = `Error: ${message}`;
                    statusBar.classList.add('error');
                    setTimeout(() => {
                        statusBar.classList.remove('error');
                    }, 5000);
                } else {
                    statusBar.textContent = message;
                    statusBar.className = 'status-bar';
                }
            }
            if (window.__TAURI__ && window.__TAURI__.core) {
                window.__TAURI__.core.invoke('add_log_entry', {level: normalizedLevel, message, module})
                    .catch(error => {
                        console.error(`Failed to log action: ${error.message}`);
                    });
            }
        }
    }

    showError(message) {
        this.logAction('ERROR', message);
    }

    setDependencies(modManager, settingsManager) {
        this.modManager = modManager;
        this.settingsManager = settingsManager; // Added to access settings in logAction
    }
}