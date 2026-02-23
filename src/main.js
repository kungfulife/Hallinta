import { state } from './js/state.js';
import { ModManager } from './js/modManager.js';
import { PresetManager } from './js/presetManager.js';
import { SettingsManager } from './js/settingsManager.js';
import { UIManager } from './js/uiManager.js';
import { BackupManager } from './js/backupManager.js';
import { SaveMonitorManager } from './js/saveMonitorManager.js';
import { GalleryManager } from './js/galleryManager.js';
import { SelectEnhancer } from './js/selectEnhancer.js';
import { setupEventHandlers } from './js/eventHandlers.js';

// Instantiate managers
const uiManager = new UIManager();
const modManager = new ModManager(uiManager);
const settingsManager = new SettingsManager(modManager, uiManager);
const presetManager = new PresetManager(uiManager, modManager, settingsManager);
const backupManager = new BackupManager(uiManager, modManager, settingsManager, presetManager);
const saveMonitorManager = new SaveMonitorManager(uiManager, settingsManager);
const galleryManager = new GalleryManager(uiManager, settingsManager, presetManager);
const selectEnhancer = new SelectEnhancer(uiManager);
window.selectEnhancer = selectEnhancer;

// Set the circular dependency
uiManager.setDependencies(modManager, settingsManager);
presetManager.setGalleryManager(galleryManager);

setupEventHandlers(uiManager, modManager, presetManager, settingsManager, backupManager, saveMonitorManager, galleryManager, selectEnhancer);
