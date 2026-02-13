import { state } from './js/state.js';
import { ModManager } from './js/modManager.js';
import { PresetManager } from './js/presetManager.js';
import { SettingsManager } from './js/settingsManager.js';
import { UIManager } from './js/uiManager.js';
import { PhraseManager } from './js/phraseManager.js';
import { BackupManager } from './js/backupManager.js';
import { setupEventHandlers } from './js/eventHandlers.js';

// Instantiate managers
const uiManager = new UIManager();
const modManager = new ModManager(uiManager);
const settingsManager = new SettingsManager(modManager, uiManager);
const presetManager = new PresetManager(uiManager, modManager, settingsManager);
const backupManager = new BackupManager(uiManager, modManager, settingsManager, presetManager);

// Set the circular dependency
uiManager.setDependencies(modManager, settingsManager);

state.phraseManager = new PhraseManager();
setupEventHandlers(uiManager, modManager, presetManager, settingsManager, backupManager);
