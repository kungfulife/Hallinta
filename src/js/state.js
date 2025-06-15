export function deepCopyMods(mods) {
    return mods.map(mod => ({...mod}));
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
    isModalVisible: false
};
