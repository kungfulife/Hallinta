window.changeView = function(view) {
  const mainPage = document.getElementById('main-page');
  const settingsPage = document.getElementById('settings-page');
  const appHeader = document.getElementById('app-header');
  if (mainPage && settingsPage && appHeader) {
    mainPage.style.display = view === 'main' ? 'flex' : 'none';
    settingsPage.style.display = view === 'settings' ? 'block' : 'none';
    appHeader.style.display = view === 'main' ? 'block' : 'none';
  }
};

window.changeDirectory = async function(type) {
  const statusBar = document.getElementById('status-bar');
  try {
    if (window.__TAURI__) {
      const { open } = await import('@tauri-apps/api/dialog');
      const selected = await open({ directory: true });
      if (selected) {
        document.getElementById(type + '-dir').value = selected;
        statusBar.textContent = `Selected directory for ${type}`;
      }
    } else {
      console.log(`Mock: Change directory for ${type}`);
      statusBar.textContent = `Mock: Selected directory for ${type}`;
    }
  } catch (error) {
    statusBar.textContent = `Error selecting directory: ${error.message}`;
  }
};

window.openDirectory = async function(type) {
  const statusBar = document.getElementById('status-bar');
  try {
    if (window.__TAURI__) {
      const { open } = await import('@tauri-apps/api/shell');
      const dir = document.getElementById(type + '-dir').value;
      if (dir) {
        await open(dir);
        statusBar.textContent = `Opened ${type} directory`;
      } else {
        statusBar.textContent = `No directory set for ${type}`;
      }
    } else {
      console.log(`Mock: Open directory for ${type}`);
      statusBar.textContent = `Mock: Opened ${type} directory`;
    }
  } catch (error) {
    statusBar.textContent = `Error opening directory: ${error.message}`;
  }
};

window.resetToDefaults = async function() {
  const statusBar = document.getElementById('status-bar');
  try {
    if (window.__TAURI__) {
      const { invoke } = window.__TAURI__.core;
      const defaultNoitaDir = await invoke('get_noita_config_path');
      document.getElementById('noita-dir').value = defaultNoitaDir;
    } else {
      document.getElementById('noita-dir').value = 'C:\\Users\\Default\\AppData\\LocalLow\\Nolla_Games_Noita\\save00\\mod_config.xml';
    }
    document.getElementById('entangled-dir').value = '';
    statusBar.textContent = 'Reset to defaults';
  } catch (error) {
    statusBar.textContent = `Error resetting defaults: ${error.message}`;
  }
};

window.saveAndClose = async function() {
  const statusBar = document.getElementById('status-bar');
  try {
    if (window.__TAURI__) {
      const { writeTextFile, BaseDirectory } = await import('@tauri-apps/api/fs');
      const { appConfigDir } = await import('@tauri-apps/api/path');
      const noitaDir = document.getElementById('noita-dir').value;
      const entangledDir = document.getElementById('entangled-dir').value;
      const config = { noitaDir, entangledDir };
      const configDir = await appConfigDir();
      await writeTextFile(`${configDir}Halinta.config`, JSON.stringify(config));
      statusBar.textContent = `Saved list - Total Mods: 76`;
    } else {
      console.log('Mock: Saved config', { noitaDir: document.getElementById('noita-dir').value, entangledDir: document.getElementById('entangled-dir').value });
      statusBar.textContent = `Mock: Saved list - Total Mods: 76`;
    }
    changeView('main');
  } catch (error) {
    statusBar.textContent = `Error saving: ${error.message}`;
  }
};

document.addEventListener('DOMContentLoaded', () => {
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
      },
      onMove: () => true,
    });
  }

  const contextMenu = document.getElementById('context-menu');
  if (list) {
    list.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      contextMenu.style.display = 'block';
      contextMenu.style.left = e.pageX + 'px';
      contextMenu.style.top = e.pageY + 'px';
    });
  }

  document.addEventListener('click', () => contextMenu.style.display = 'none');
});