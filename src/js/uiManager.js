import { state } from './state.js';

export class UIManager {
    constructor(modManager) {
        this.modManager = modManager;
        this.statusBar = document.getElementById('status-bar');
    }

    changeView(view) {
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

            // this.statusBar.style.display = 'block';

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

            // this.statusBar.style.display = 'none';
        }
    }

    renderModList() {
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
    }

    updateModCount() {
        const modCount = document.getElementById('mod-count');
        const enabledCount = state.currentMods.filter(mod => mod.enabled).length;
        modCount.textContent = `Total Mods: ${state.currentMods.length} (${enabledCount} enabled)`;
    }

    filterMods() {
        const searchTerm = document.querySelector('.search-bar').value.toLowerCase();
        const modItems = document.querySelectorAll('.mod-item');

        modItems.forEach(item => {
            const modName = item.querySelector('.mod-name').textContent.toLowerCase();
            item.style.display = modName.includes(searchTerm) ? 'flex' : 'none';
        });
    }

    toggleDarkMode() {
        const checkbox = document.getElementById('dark-mode-checkbox');
        state.isDarkMode = checkbox.checked;
        this.applyDarkMode();
    }

    applyDarkMode() {
        document.body.classList.toggle('dark-mode', state.isDarkMode);
    }

    showConfirmModal(message, onConfirm, onCancel) {
        const modal = document.createElement('div');
        modal.className = 'custom-modal';
        modal.innerHTML = `
            <div class="modal-content">
                <p>${message}</p>
                <div class="modal-buttons">
                    <button id="modal-confirm">Confirm</button>
                    <button id="modal-cancel">Cancel</button>
                </div>
            </div>
        `;

        document.body.appendChild(modal);

        document.getElementById('modal-confirm').addEventListener('click', () => {
            onConfirm();
            document.body.removeChild(modal);
        });

        document.getElementById('modal-cancel').addEventListener('click', () => {
            onCancel();
            document.body.removeChild(modal);
        });
    }

    showInputModal(message, defaultValue, onConfirm, onCancel) {
        const modal = document.createElement('div');
        modal.className = 'custom-modal';
        modal.innerHTML = `
            <div class="modal-content">
                <p>${message}</p>
                <input type="number" id="modal-input" value="${defaultValue}" min="1" max="${state.currentMods.length}">
                <div class="modal-buttons">
                    <button id="modal-confirm">OK</button>
                    <button id="modal-cancel">Cancel</button>
                </div>
            </div>
        `;

        document.body.appendChild(modal);

        document.getElementById('modal-confirm').addEventListener('click', () => {
            const input = document.getElementById('modal-input').value;
            onConfirm(input);
            document.body.removeChild(modal);
        });

        document.getElementById('modal-cancel').addEventListener('click', () => {
            onCancel();
            document.body.removeChild(modal);
        });
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
                `Are you sure you want to delete the mod "${modName}"?`,
                () => {
                    this.modManager.deleteMod(state.contextMenuTarget);
                    this.statusBar.textContent = `Deleted mod: ${modName}`;
                    document.getElementById('mod-context-menu').style.display = 'none';
                },
                () => {
                    this.statusBar.textContent = `Deletion of "${modName}" canceled`;
                    document.getElementById('mod-context-menu').style.display = 'none';
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
                (input) => {
                    const newIndex = parseInt(input) - 1;
                    if (isNaN(newIndex) || newIndex < 0 || newIndex >= state.currentMods.length) {
                        this.statusBar.textContent = `Invalid position for "${modName}"`;
                    } else {
                        this.modManager.reorderMod(state.contextMenuTarget, newIndex);
                        this.statusBar.textContent = `Moved "${modName}" to position ${newIndex + 1}`;
                    }
                    document.getElementById('mod-context-menu').style.display = 'none';
                },
                () => {
                    this.statusBar.textContent = `Reordering of "${modName}" canceled`;
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
                    await window.__TAURI__.core.invoke('open_workshop_item', { workshopId: mod.workshopId });
                    this.statusBar.textContent = `Opened workshop page for ${mod.name}`;
                } catch (error) {
                    console.error('Error opening workshop:', error);
                    const url = `https://steamcommunity.com/sharedfiles/filedetails/?id=${mod.workshopId}`;
                    window.open(url, '_blank');
                    this.statusBar.textContent = `Opened workshop page for ${mod.name}`;
                }
            } else {
                this.statusBar.textContent = 'No workshop ID available for this mod';
            }
            document.getElementById('mod-context-menu').style.display = 'none';
        }
    }

    async copyWorkshopLink() {
        if (state.contextMenuTarget !== null) {
            const mod = state.currentMods[state.contextMenuTarget];
            if (mod.workshopId !== '0') {
                const url = `https://steamcommunity.com/sharedfiles/filedetails/?id=${mod.workshopId}`;
                try {
                    await navigator.clipboard.writeText(url);
                    this.statusBar.textContent = `Copied workshop link for ${mod.name}`;
                } catch (error) {
                    console.error('Error copying to clipboard:', error);
                    this.statusBar.textContent = 'Failed to copy workshop link';
                }
            } else {
                this.statusBar.textContent = 'No workshop ID available for this mod';
            }
            document.getElementById('mod-context-menu').style.display = 'none';
        }
    }
}
