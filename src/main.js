import { state } from './js/state.js';
import { ModManager } from './js/modManager.js';
import { PresetManager } from './js/presetManager.js';
import { SettingsManager } from './js/settingsManager.js';
import { UIManager } from './js/uiManager.js';
import { PhraseManager } from './js/phraseManager.js';
import { setupEventHandlers } from './js/eventHandlers.js';

const uiManager = new UIManager(null);
const modManager = new ModManager(uiManager);
uiManager.modManager = modManager;
const presetManager = new PresetManager(uiManager, modManager);
const settingsManager = new SettingsManager(modManager, uiManager);

state.phraseManager = new PhraseManager();

setupEventHandlers(uiManager, modManager, presetManager, settingsManager);