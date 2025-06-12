let isDarkMode = false;
let currentMods = [];
let isAppFocused = true;
let lastKnownModOrder = [];
let phraseManager;

// Preset Management
let currentPresets = {'Default': []};
let selectedPreset = 'Default';

window.changeView = function (view) {
    const mainPage = document.getElementById('main-page');
    const settingsPage = document.getElementById('settings-page');
    const appHeader = document.getElementById('app-header');
    const presetSelector = document.getElementById('preset-selector');
    const statusBar = document.getElementById('status-bar');

    if (mainPage && settingsPage && appHeader) {
        if (view === 'main') {
            // Show main page elements
            mainPage.style.display = 'flex';
            settingsPage.style.display = 'none';
            appHeader.style.display = 'flex'; // Changed from 'block' to 'flex'

            if (presetSelector) {
                presetSelector.style.display = 'block';
            }
            if (statusBar) {
                statusBar.style.display = 'block';
            }

            // Force layout recalculation to prevent button displacement
            setTimeout(() => {
                appHeader.style.display = 'flex';
                // Trigger reflow
                appHeader.offsetHeight;
            }, 0);

        } else if (view === 'settings') {
            // Show settings page
            mainPage.style.display = 'none';
            settingsPage.style.display = 'block';
            appHeader.style.display = 'none';

            if (presetSelector) {
                presetSelector.style.display = 'none';
            }
            if (statusBar) {
                statusBar.style.display = 'none';
            }
        }
    }
};


window.changeDirectory = async function (type) {
    const statusBar = document.getElementById('status-bar');
    try {
        if (window.__TAURI__ && window.__TAURI__.dialog) {
            const selected = await window.__TAURI__.dialog.open({
                directory: true,
                multiple: false
            });
            if (selected) {
                document.getElementById(type + '-dir').value = selected;
                statusBar.className = 'status-bar';
                statusBar.textContent = `Selected directory for ${type}: ${selected}`;

                if (type === 'noita') {
                    await loadModConfigFromDirectory(selected);
                }
            }
        }
    } catch (error) {
        console.error('Directory selection error:', error);
        statusBar.className = 'status-bar';
        statusBar.textContent = `Error selecting directory: ${error.message}`;
    }
};

// TODO
window.openDirectory = async function (type) {
};

window.resetToDefaults = async function () {
    const statusBar = document.getElementById('status-bar');

    try {
        if (!window.__TAURI__ && !window.__TAURI__.core) {
            console.error('Tauri is not initialized');
            statusBar.textContent = `Error resetting defaults: Tauri is not initialized`;
            return
        }


        const defaultNoitaDir = await window.__TAURI__.core.invoke('get_noita_save_path');
        document.getElementById('noita-dir').value = defaultNoitaDir;
        await loadModConfigFromDirectory(defaultNoitaDir);

        document.getElementById('entangled-dir').value = '';
        statusBar.textContent = 'Successfully reset to defaults';
    } catch (invokeError) {
        console.error('Reset defaults error:', invokeError);
        statusBar.textContent = `Error resetting defaults: ${invokeError.message}`;
    }


};

window.saveAndClose = async function () {
    const statusBar = document.getElementById('status-bar');
    try {
        const noitaDir = document.getElementById('noita-dir').value;
        const entangledDir = document.getElementById('entangled-dir').value;

        if (window.__TAURI__ && window.__TAURI__.core) {
            const settings = {
                noita_dir: noitaDir,
                entangled_dir: entangledDir,
                dark_mode: isDarkMode
            };

            const presetsForSave = {};
            Object.keys(currentPresets).forEach(presetName => {
                presetsForSave[presetName] = currentPresets[presetName].map(mod => ({
                    name: mod.name,
                    enabled: mod.enabled,
                    workshop_id: mod.workshopId || '0',
                    settings_fold_open: mod.settingsFoldOpen || false
                }));
            });

            try {
                await window.__TAURI__.core.invoke('save_settings', {settings: settings});
                await window.__TAURI__.core.invoke('save_presets', {presets: presetsForSave});
                statusBar.textContent = `Configuration saved successfully`;
            } catch (error) {
                console.error('Save error:', error);
                statusBar.textContent = `Error saving configuration: ${error.message}`;
            }
        }

        changeView('main');
    } catch (error) {
        console.error('Save and close error:', error);
        statusBar.textContent = `Critical error during save: ${error.message}`;
        changeView('main');
    }
};

// XML Parsing and Mod Management
async function loadModConfigFromDirectory(directory) {
    const statusBar = document.getElementById('status-bar');
    try {
        if (window.__TAURI__ && window.__TAURI__.core) {
            const xmlContent = await window.__TAURI__.core.invoke('read_mod_config', {directory: directory});
            parseModConfig(xmlContent, true);
            statusBar.textContent = `Loaded ${currentMods.length} mods from mod_config.xml`;
            updateModCount();
        }
    } catch (error) {
        console.error('Error loading mod config:', error);
        statusBar.textContent = `Error loading mod_config.xml: ${error.message}`;
    }
}

function parseModConfig(xmlContent, isInitialLoad = false) {
    try {
        const parser = new DOMParser();
        const xmlDoc = parser.parseFromString(xmlContent, 'text/xml');

        const parserError = xmlDoc.querySelector('parsererror');
        if (parserError) {
            throw new Error('XML parsing failed: ' + parserError.textContent);
        }

        const mods = xmlDoc.querySelectorAll('Mod');

        currentMods = Array.from(mods).map((mod, index) => ({
            name: mod.getAttribute('name') || 'Unknown Mod',
            enabled: mod.getAttribute('enabled') === '1',
            workshopId: mod.getAttribute('workshop_item_id') || '0',
            settingsFoldOpen: mod.getAttribute('settings_fold_open') === '1',
            index: index
        }));

        lastKnownModOrder = [...currentMods];
        currentPresets[selectedPreset] = [...currentMods];
        renderModList();
        updateModCount();
    } catch (error) {
        console.error('Error parsing XML:', error);
        document.getElementById('status-bar').textContent = `Error parsing XML: ${error.message}`;
    }
}

function renderModList() {
    const modList = document.getElementById('mod-list');
    modList.innerHTML = '';

    currentMods.forEach((mod, index) => {
        const li = document.createElement('li');
        li.className = 'mod-item';
        li.setAttribute('data-index', index);

        // Make the entire item clickable
        li.addEventListener('click', (e) => {
            // Prevent toggle when right-clicking for context menu
            if (e.button !== 2) {
                e.stopPropagation();
                toggleModAtIndex(index);
                renderModList(); // Re-render to update visual state
            }
        });

        // Add enabled/disabled class for visual styling
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
        typeSpan.textContent = mod.workshopId !== "0" ? `Workshop ID: ${mod.workshopId}` : "Local Mod";

        if (mod.workshopId !== "0") {
            typeSpan.classList.add('workshop');
        } else {
            typeSpan.classList.add('local');
        }

        infoDiv.appendChild(nameSpan);
        infoDiv.appendChild(typeSpan);

        // Create visual-only checkbox
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

function updateModCount() {
    const modCount = document.getElementById('mod-count');
    const enabledCount = currentMods.filter(mod => mod.enabled).length;
    modCount.textContent = `Total Mods: ${currentMods.length} (${enabledCount} enabled)`;
}

window.filterMods = function () {
    const searchTerm = document.querySelector('.search-bar').value.toLowerCase();
    const modItems = document.querySelectorAll('.mod-item');

    modItems.forEach(item => {
        const modName = item.querySelector('.mod-name').textContent.toLowerCase();
        if (modName.includes(searchTerm)) {
            item.style.display = 'flex';
        } else {
            item.style.display = 'none';
        }
    });
};

window.toggleModAtIndex = function (index) {
    currentMods[index].enabled = !currentMods[index].enabled;
    updateModCount();
    saveModConfigToFile();

    const statusBar = document.getElementById('status-bar');
    statusBar.className = 'status-bar';
    statusBar.textContent = `${currentMods[index].name} ${currentMods[index].enabled ? 'enabled' : 'disabled'}`;
}

async function saveModConfigToFile() {
    try {
        const noitaDir = document.getElementById('noita-dir').value;

        const xmlContent = generateModConfigXML();
        await window.__TAURI__.core.invoke('write_mod_config', {
            directory: noitaDir,
            content: xmlContent
        });
        currentPresets[selectedPreset] = [...currentMods];
        console.log('Mod config saved successfully');
    } catch (error) {
        console.error('Error saving mod config:', error);
        document.getElementById('status-bar').textContent = `Error saving mod_config.xml: ${error.message}`;
    }
}

function generateModConfigXML() {
    let xml = '<Mods>\n';
    currentMods.forEach(mod => {
        xml += `  <Mod enabled="${mod.enabled ? '1' : '0'}" name="${mod.name}" settings_fold_open="${mod.settingsFoldOpen ? '1' : '0'}" workshop_item_id="${mod.workshopId}" >\n  </Mod>\n`;
    });
    xml += '</Mods>';
    return xml;
}

// Context Menu Functions
let contextMenuTarget = null;

window.toggleModEnabled = function () {
    if (contextMenuTarget !== null) {
        toggleModAtIndex(contextMenuTarget);
        document.getElementById('context-menu').style.display = 'none';
    }
};

window.deleteMod = function () {
    if (contextMenuTarget !== null) {
        const modName = currentMods[contextMenuTarget].name;
        currentMods.splice(contextMenuTarget, 1);
        renderModList();
        updateModCount();
        saveModConfigToFile();
        document.getElementById('status-bar').textContent = `Deleted mod: ${modName}`;
        document.getElementById('context-menu').style.display = 'none';
    }
};

window.reorderMod = function () {
    document.getElementById('status-bar').textContent = 'Drag and drop to reorder mods';
    document.getElementById('context-menu').style.display = 'none';
};

window.openWorkshop = async function () {
    if (contextMenuTarget !== null) {
        const mod = currentMods[contextMenuTarget];
        if (mod.workshopId !== '0' && window.__TAURI__) {
            try {
                await window.__TAURI__.core.invoke('open_workshop_item', {workshopId: mod.workshopId});
                document.getElementById('status-bar').textContent = `Opened workshop page for ${mod.name}`;
            } catch (error) {
                console.error('Error opening workshop:', error);
                const url = `https://steamcommunity.com/sharedfiles/filedetails/?id=${mod.workshopId}`;
                window.open(url, '_blank');
                document.getElementById('status-bar').textContent = `Opened workshop page for ${mod.name}`;
            }
        } else {
            document.getElementById('status-bar').textContent = 'No workshop ID available for this mod';
        }
        document.getElementById('context-menu').style.display = 'none';
    }
};

window.copyWorkshopLink = async function () {
    if (contextMenuTarget !== null) {
        const mod = currentMods[contextMenuTarget];
        if (mod.workshopId !== '0') {
            const url = `https://steamcommunity.com/sharedfiles/filedetails/?id=${mod.workshopId}`;
            try {
                await navigator.clipboard.writeText(url);
                document.getElementById('status-bar').textContent = `Copied workshop link for ${mod.name}`;
            } catch (error) {
                console.error('Error copying to clipboard:', error);
                document.getElementById('status-bar').textContent = 'Failed to copy workshop link';
            }
        } else {
            document.getElementById('status-bar').textContent = 'No workshop ID available for this mod';
        }
        document.getElementById('context-menu').style.display = 'none';
    }
};

// Export/Import Functions
// TODO
window.exportModList = async function () {
};

// Backup Functions

// TODO
window.backupMonitor = async function () {
};

// TODO
window.restoreBackup = function () {
};

// TODO
window.createBackup = function () {
};

// Split Button Functions

// TODO
window.importRegular = function () {
};

// TODO
window.importSteam = function () {
};

// Dark Mode
window.toggleDarkMode = function () {
    const checkbox = document.getElementById('dark-mode-checkbox');
    isDarkMode = checkbox.checked;
    applyDarkMode();
};

function applyDarkMode() {
    if (isDarkMode) {
        document.body.classList.add('dark-mode');
    } else {
        document.body.classList.remove('dark-mode');
    }
}

// Preset Management
window.loadPresets = function () {
    const selector = document.getElementById('preset-dropdown');
    selector.innerHTML = '';

    const createOption = document.createElement('option');
    createOption.value = '__create_new__';
    createOption.textContent = '+ Create New Preset';
    selector.appendChild(createOption);

    Object.keys(currentPresets).forEach(preset => {
        const option = document.createElement('option');
        option.value = preset;
        option.textContent = preset;
        if (preset === selectedPreset) {
            option.selected = true;
        }
        selector.appendChild(option);
    });
};

window.onPresetChange = function () {
    const selector = document.getElementById('preset-dropdown');
    const statusBar = document.getElementById('status-bar');

    if (selector.value === '__create_new__') {
        const newName = prompt('Enter name for new preset:', `Preset ${Object.keys(currentPresets).length + 1}`);
        if (newName && !currentPresets[newName]) {
            currentPresets[newName] = [...currentMods];
            selectedPreset = newName;
            loadPresets();
            statusBar.textContent = `Created new preset: ${newName}`;
        } else {
            selector.value = selectedPreset;
        }
    } else {
        selectedPreset = selector.value;
        currentMods = [...currentPresets[selectedPreset]];
        renderModList();
        updateModCount();
        statusBar.textContent = `Switched to preset: ${selectedPreset}`;
    }
};

window.deleteCurrentPreset = function () {
    const statusBar = document.getElementById('status-bar');

    if (selectedPreset === 'Default') {
        statusBar.textContent = 'Cannot delete the Default preset';
        return;
    }

    if (window.confirm(`Delete preset "${selectedPreset}"?`)) {
        delete currentPresets[selectedPreset];
        selectedPreset = 'Default';
        currentMods = [...currentPresets[selectedPreset]];
        renderModList();
        updateModCount();
        loadPresets();
        statusBar.textContent = `Deleted preset`;
    }
};

window.renameCurrentPreset = function () {
    const statusBar = document.getElementById('status-bar');

    if (selectedPreset === 'Default') {
        statusBar.textContent = 'Cannot rename the Default preset';
        return;
    }

    const newName = prompt('Enter new name:', selectedPreset);
    if (newName && newName !== selectedPreset && !currentPresets[newName]) {
        currentPresets[newName] = currentPresets[selectedPreset];
        delete currentPresets[selectedPreset];
        selectedPreset = newName;
        loadPresets();
        statusBar.textContent = `Renamed preset to: ${newName}`;
    }
};

// App Focus Detection
window.addEventListener('focus', () => {
    isAppFocused = true;
});

window.addEventListener('blur', () => {
    isAppFocused = false;
});

// Load Configuration
window.loadConfig = async function () {
    try {
        if (window.__TAURI__ && window.__TAURI__.core) {
            try {
                const settings = await window.__TAURI__.core.invoke('load_settings');
                const presets = await window.__TAURI__.core.invoke('load_presets');

                document.getElementById('noita-dir').value = settings.noita_dir;
                document.getElementById('entangled-dir').value = settings.entangled_dir;
                isDarkMode = settings.dark_mode;
                document.getElementById('dark-mode-checkbox').checked = isDarkMode;
                applyDarkMode();

                currentPresets = {};
                Object.keys(presets).forEach(presetName => {
                    currentPresets[presetName] = presets[presetName].map(mod => ({
                        name: mod.name,
                        enabled: mod.enabled,
                        workshopId: mod.workshop_id,
                        settingsFoldOpen: mod.settings_fold_open,
                        index: 0
                    }));
                });

                if (settings.noita_dir) {
                    await loadModConfigFromDirectory(settings.noita_dir);
                }

                console.log('Configuration loaded successfully');
            } catch (error) {
                console.error('Error loading from files, creating defaults:', error);

                const defaultSettings = {
                    noita_dir: await window.__TAURI__.core.invoke('get_noita_save_path'),
                    entangled_dir: '',
                    dark_mode: false
                };

                const defaultPresets = {
                    'Default': []
                };

                try {
                    await window.__TAURI__.core.invoke('save_settings', {settings: defaultSettings});
                    await window.__TAURI__.core.invoke('save_presets', {presets: defaultPresets});

                    document.getElementById('noita-dir').value = defaultSettings.noita_dir;
                    document.getElementById('entangled-dir').value = defaultSettings.entangled_dir;
                    isDarkMode = defaultSettings.dark_mode;
                    document.getElementById('dark-mode-checkbox').checked = isDarkMode;
                    currentPresets = {'Default': []};

                    await loadModConfigFromDirectory(defaultSettings.noita_dir);

                    document.getElementById('status-bar').textContent = 'Created default configuration';
                } catch (saveError) {
                    console.error('Error creating defaults:', saveError);
                }
            }
        }
    } catch (error) {
        console.error('Error loading configuration:', error);
    }
};

async function setupFileWatcher(filePath) {
    console.log('File watcher setup for:', filePath);
}

document.addEventListener('DOMContentLoaded', () => {
    phraseManager = new PhraseManager();

    loadConfig();
    loadPresets();

    setTimeout(() => {
        phraseManager.startRandomPhrases();
    }, 2000);

    const list = document.getElementById('mod-list');
    if (list) {
        new Sortable(list, {
            animation: 150,
            ghostClass: 'sortable-ghost',
            forceFallback: true,
            onEnd: (evt) => {
                const movedMod = currentMods.splice(evt.oldIndex, 1)[0];
                currentMods.splice(evt.newIndex, 0, movedMod);

                currentMods.forEach((mod, index) => {
                    mod.index = index;
                });

                renderModList();
                updateModCount();

                saveModConfigToFile();

                const statusBar = document.getElementById('status-bar');
                if (statusBar) {
                    statusBar.textContent = `Mod reordered: ${evt.oldIndex + 1} â†’ ${evt.newIndex + 1}`;
                }
            },
            onMove: () => true,
        });
    }

    const contextMenu = document.getElementById('context-menu');
    if (list && contextMenu) {
        list.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            const modItem = e.target.closest('.mod-item');
            if (modItem) {
                contextMenuTarget = parseInt(modItem.getAttribute('data-index'));
                contextMenu.style.display = 'block';
                contextMenu.style.left = e.pageX + 'px';
                contextMenu.style.top = e.pageY + 'px';
            }
        });
    }

    if (contextMenu) {
        document.addEventListener('click', () => {
            contextMenu.style.display = 'none';
        });
    }
});

window.addEventListener('beforeunload', () => {
    if (phraseManager) {
        phraseManager.stopRandomPhrases();
    }
});
