export function deepCopyMods(mods) {
    return mods.map(mod => ({...mod}));
}

export function buildPresetsForSave(presets) {
    const presetsForSave = {};
    Object.keys(presets).forEach((presetName) => {
        presetsForSave[presetName] = presets[presetName].map((mod) => ({
            name: mod.name,
            enabled: mod.enabled,
            workshop_id: mod.workshopId || '0',
            settings_fold_open: mod.settingsFoldOpen || false
        }));
    });
    return presetsForSave;
}

export const state = {
    currentMods: [],
    currentPresets: {"Default": []},
    selectedPreset: "Default",
    isDarkMode: false,
    lastKnownModOrder: [],
    isAppFocused: true,
    phraseManager: null,
    contextMenuTarget: null,
    fileWatcher: null,
    lastModifiedTime: 0,
    isReordering: false,
    pendingReorder: false,
    logger: null,
    isModalVisible: false,
    backupInProgress: false,
    isRestoring: false,
    galleryView: false,
    logAutoRefreshInterval: null,
    logFilters: {
        debug: true,
        info: true,
        warn: true,
        error: true,
        search: ''
    }
};
