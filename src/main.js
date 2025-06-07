window.changeView = function(view) {
  const mainPage = document.getElementById('main-page');
  const settingsPage = document.getElementById('settings-page');
  const appHeader = document.getElementById('app-header');
  const presetSelector = document.getElementById('preset-selector');
  const statusBar = document.getElementById('status-bar');

  if (mainPage && settingsPage && appHeader) {
    mainPage.style.display = view === 'main' ? 'flex' : 'none';
    settingsPage.style.display = view === 'settings' ? 'block' : 'none';
    appHeader.style.display = view === 'main' ? 'block' : 'none';

    if (presetSelector) {
      presetSelector.style.display = view === 'main' ? 'block' : 'none';
    }

    // Hide status bar during settings view
    if (statusBar) {
      statusBar.style.display = view === 'settings' ? 'none' : 'block';
    }
  }
};

window.changeDirectory = async function(type) {
  const statusBar = document.getElementById('status-bar');
  try {
    if (window.__TAURI__ && window.__TAURI__.dialog) {
      const selected = await window.__TAURI__.dialog.open({
        directory: true,
        multiple: false
      });
      if (selected) {
        document.getElementById(type + '-dir').value = selected;
        statusBar.textContent = `Selected directory for ${type}: ${selected}`;
      }
    } else {
      console.log(`Mock: Change directory for ${type}`);
      statusBar.textContent = `Mock: Selected directory for ${type}`;
    }
  } catch (error) {
    console.error('Directory selection error:', error);
    statusBar.textContent = `Error selecting directory: ${error.message}`;
  }
};

window.openDirectory = async function(type) {
  const statusBar = document.getElementById('status-bar');
  try {
    if (window.__TAURI__ && window.__TAURI__.shell) {
      const dir = document.getElementById(type + '-dir').value;
      if (dir) {
        await window.__TAURI__.shell.open(dir);
        statusBar.textContent = `Opened ${type} directory: ${dir}`;
      } else {
        statusBar.textContent = `No directory set for ${type}`;
      }
    } else {
      console.log(`Mock: Open directory for ${type}`);
      statusBar.textContent = `Mock: Opened ${type} directory`;
    }
  } catch (error) {
    console.error('Directory open error:', error);
    statusBar.textContent = `Error opening directory: ${error.message}`;
  }
};

window.resetToDefaults = async function() {
  const statusBar = document.getElementById('status-bar');
  try {
    if (window.__TAURI__ && window.__TAURI__.core) {
      try {
        const defaultNoitaDir = await window.__TAURI__.core.invoke('get_noita_config_path');
        document.getElementById('noita-dir').value = defaultNoitaDir;
      } catch (invokeError) {
        document.getElementById('noita-dir').value = 'C:\\Users\\Default\\AppData\\LocalLow\\Nolla_Games_Noita\\save00\\mod_config.xml';
      }
    } else {
      document.getElementById('noita-dir').value = 'C:\\Users\\Default\\AppData\\LocalLow\\Nolla_Games_Noita\\save00\\mod_config.xml';
    }
    document.getElementById('entangled-dir').value = '';
    statusBar.textContent = 'Successfully reset to defaults';
  } catch (error) {
    console.error('Reset defaults error:', error);
    statusBar.textContent = `Error resetting defaults: ${error.message}`;
  }
};

window.saveAndClose = async function() {
  const statusBar = document.getElementById('status-bar');
  try {
    const noitaDir = document.getElementById('noita-dir').value;
    const entangledDir = document.getElementById('entangled-dir').value;

    if (window.__TAURI__ && window.__TAURI__.fs && window.__TAURI__.path && window.__TAURI__.core) {
      const config = { noitaDir, entangledDir };

      try {
        // Get executable directory
        const exeDir = await window.__TAURI__.core.invoke('get_exe_dir');
        const configPath = await window.__TAURI__.path.join(exeDir, 'Halinta.config');

        // Write config file next to executable
        await window.__TAURI__.fs.writeTextFile(configPath, JSON.stringify(config, null, 2));
        statusBar.textContent = `Configuration saved successfully`;
      } catch (saveError) {
        console.error('Save error:', saveError);
        statusBar.textContent = `Error saving configuration: ${saveError.message}`;
      }
    } else {
      console.log('Mock: Saved config', { noitaDir, entangledDir });
      statusBar.textContent = `Mock: Configuration saved`;
    }

    changeView('main');
  } catch (error) {
    console.error('Save and close error:', error);
    statusBar.textContent = `Critical error during save: ${error.message}`;
    changeView('main');
  }
};

// Load configuration on app start
window.loadConfig = async function() {
  try {
    if (window.__TAURI__ && window.__TAURI__.fs && window.__TAURI__.path) {
      const exeDir = await window.__TAURI__.core.invoke('get_exe_dir');
      const configPath = await window.__TAURI__.path.join(exeDir, 'Halinta.config');

      try {
        const configContent = await window.__TAURI__.fs.readTextFile(configPath);
        const config = JSON.parse(configContent);

        if (config.noitaDir) {
          document.getElementById('noita-dir').value = config.noitaDir;
        }
        if (config.entangledDir) {
          document.getElementById('entangled-dir').value = config.entangledDir;
        }

        console.log('Configuration loaded successfully');
      } catch (readError) {
        console.log('No existing configuration found, using defaults');
      }
    }
  } catch (error) {
    console.error('Error loading configuration:', error);
  }
};

// Preset Management Functions
let currentPresets = ['Default'];
let selectedPreset = 'Default';

window.loadPresets = function() {
  const selector = document.getElementById('preset-dropdown');
  selector.innerHTML = '';

  const createOption = document.createElement('option');
  createOption.value = '__create_new__';
  createOption.textContent = '+ Create New Preset';
  selector.appendChild(createOption);

  currentPresets.forEach(preset => {
    const option = document.createElement('option');
    option.value = preset;
    option.textContent = preset;
    if (preset === selectedPreset) {
      option.selected = true;
    }
    selector.appendChild(option);
  });
};

window.onPresetChange = function() {
  const selector = document.getElementById('preset-dropdown');
  const statusBar = document.getElementById('status-bar');

  if (selector.value === '__create_new__') {
    const newName = prompt('Enter name for new preset:', `Preset ${currentPresets.length + 1}`);
    if (newName && !currentPresets.includes(newName)) {
      currentPresets.push(newName);
      selectedPreset = newName;
      loadPresets();
      statusBar.textContent = `Created new preset: ${newName}`;
    } else {
      selector.value = selectedPreset;
    }
  } else {
    selectedPreset = selector.value;
    statusBar.textContent = `Switched to preset: ${selectedPreset}`;
  }
};

window.deleteCurrentPreset = function() {
  const statusBar = document.getElementById('status-bar');

  if (selectedPreset === 'Default') {
    statusBar.textContent = 'Cannot delete the Default preset';
    return;
  }

  // Use window.confirm instead of Tauri dialog for simple confirmations
  if (window.confirm(`Delete preset "${selectedPreset}"?`)) {
    const index = currentPresets.indexOf(selectedPreset);
    if (index > -1) {
      currentPresets.splice(index, 1);
      selectedPreset = 'Default';
      loadPresets();
      statusBar.textContent = `Deleted preset`;
    }
  }
};

window.renameCurrentPreset = function() {
  const statusBar = document.getElementById('status-bar');

  if (selectedPreset === 'Default') {
    statusBar.textContent = 'Cannot rename the Default preset';
    return;
  }

  const newName = prompt('Enter new name:', selectedPreset);
  if (newName && newName !== selectedPreset && !currentPresets.includes(newName)) {
    const index = currentPresets.indexOf(selectedPreset);
    if (index > -1) {
      currentPresets[index] = newName;
      selectedPreset = newName;
      loadPresets();
      statusBar.textContent = `Renamed preset to: ${newName}`;
    }
  }
};

document.addEventListener('DOMContentLoaded', () => {
  // Load configuration first
  loadConfig();

  // Initialize presets
  loadPresets();

  const list = document.getElementById('mod-list');
  if (list) {
    new Sortable(list, {
      animation: 150,
      ghostClass: 'sortable-ghost',
      forceFallback: true,
      onEnd: (evt) => {
        console.log(`Moved item from ${evt.oldIndex} to ${evt.newIndex}`);
        const items = Array.from(list.children).map(item => item.textContent);
        console.log('New order:', items);

        const statusBar = document.getElementById('status-bar');
        if (statusBar) {
          statusBar.textContent = `Mod reordered: ${evt.oldIndex + 1} → ${evt.newIndex + 1}`;
        }
      },
      onMove: () => true,
    });
  }

  const contextMenu = document.getElementById('context-menu');
  if (list && contextMenu) {
    list.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      contextMenu.style.display = 'block';
      contextMenu.style.left = e.pageX + 'px';
      contextMenu.style.top = e.pageY + 'px';
    });
  }

  if (contextMenu) {
    document.addEventListener('click', () => {
      contextMenu.style.display = 'none';
    });
  }
});
