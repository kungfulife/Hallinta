use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;

// ── Core Data ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModEntry {
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub workshop_id: String,
    #[serde(default)]
    pub settings_fold_open: bool,
}

// ── Settings ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub noita_dir: String,
    pub entangled_dir: String,
    pub dark_mode: bool,
    pub selected_preset: String,
    pub version: String,
    #[serde(default)]
    pub log_settings: LogSettings,
    #[serde(default)]
    pub backup_settings: BackupSettings,
    #[serde(default)]
    pub save_monitor_settings: SaveMonitorSettings,
    #[serde(default)]
    pub gallery_settings: GallerySettings,
    #[serde(default)]
    pub compact_mode: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogSettings {
    pub max_log_files: usize,
    pub max_log_size_mb: usize,
    pub log_level: String,
    pub auto_save: bool,
    #[serde(default)]
    pub collect_system_info: bool,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            max_log_files: 50,
            max_log_size_mb: 10,
            log_level: "INFO".to_string(),
            auto_save: true,
            collect_system_info: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupSettings {
    pub auto_delete_days: u32,
    pub backup_interval_minutes: u32,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            auto_delete_days: 30,
            backup_interval_minutes: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveMonitorSettings {
    pub interval_minutes: u32,
    pub max_snapshots_per_preset: usize,
    pub include_entangled: bool,
    #[serde(default)]
    pub start_in_monitor_mode: bool,
    #[serde(default = "default_keep_every_nth")]
    pub keep_every_nth: usize,
}

fn default_keep_every_nth() -> usize {
    5
}

impl Default for SaveMonitorSettings {
    fn default() -> Self {
        Self {
            interval_minutes: 3,
            max_snapshots_per_preset: 10,
            include_entangled: false,
            start_in_monitor_mode: false,
            keep_every_nth: 5,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GallerySettings {
    pub catalog_url: String,
    pub steam_path: String,
}

impl Default for GallerySettings {
    fn default() -> Self {
        Self {
            catalog_url: String::new(),
            steam_path: String::new(),
        }
    }
}

// ── Backup ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupInfo {
    pub filename: String,
    pub timestamp: String,
    pub size_bytes: u64,
    pub contains_save00: bool,
    pub contains_save01: bool,
    pub contains_presets: bool,
    #[serde(default)]
    pub contains_entangled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RestoreOptions {
    pub restore_save00: bool,
    pub restore_save01: bool,
    pub restore_presets: bool,
    #[serde(default)]
    pub restore_entangled: bool,
}

// ── Logging ────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub module: String,
}

// ── System Info ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemInfo {
    pub app_version: String,
    pub build_profile: String,
    pub rust_version: String,
    pub cargo_version: String,
    pub build_target: String,
    pub gui_framework: String,
    pub os: String,
    pub os_family: String,
    pub arch: String,
    pub logical_cpu_cores: usize,
    pub local_time: String,
    pub utc_time: String,
    pub executable_dir: String,
    pub app_data_dir: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OpenSourceLibrary {
    pub name: String,
    pub version: String,
    pub purpose: String,
    pub homepage: String,
}

// ── Gallery / Catalog ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Catalog {
    pub catalog_version: String,
    pub last_updated: String,
    pub presets: Vec<CatalogPresetEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CatalogPresetEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub mod_count: usize,
    pub version: String,
    pub checksum: String,
    pub download_url: String,
    pub thumbnail_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Save Monitor ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MonitorSnapshot {
    pub filename: String,
    pub preset_name: String,
    pub timestamp: String,
    pub size_bytes: u64,
}

// ── UI State Enums ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    ModList,
    PresetVault,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    All,
    Enabled,
    Disabled,
}

impl FilterMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Enabled => "Enabled",
            Self::Disabled => "Disabled",
        }
    }
}

// ── Modal System ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum Modal {
    Confirm {
        message: String,
        confirm_text: String,
        cancel_text: String,
        action: ConfirmAction,
        cancel_action: Option<ConfirmAction>,
    },
    Input {
        title: String,
        value: String,
        action: InputAction,
    },
    Checklist {
        title: String,
        message: String,
        items: Vec<ChecklistItem>,
        action: ChecklistAction,
    },
    Info {
        title: String,
        message: String,
    },
    Progress {
        message: String,
        progress: f32,
    },
    MissingMods {
        mods: Vec<(String, String)>, // (name, workshop_id)
        action: MissingModsAction,
    },
    SystemInfo,
    OpenSourceLibraries,
    BackupManager,
    SnapshotManager {
        preset_name: String,
    },
}

#[derive(Clone, Debug)]
pub enum ConfirmAction {
    DeletePreset,
    DeleteMod(usize),
    AcceptExternalChanges(Vec<ModEntry>),
    KeepCurrentPreset,
    OverwritePresetImport(PresetImportData),
    RenamePresetImport(PresetImportData),
    ChecksumMismatchContinue(PresetImportData),
    ExitWithSnapshot,
    ExitWithoutSnapshot,
    DeleteBackup(String),
    ClearMonitorData,
}

#[derive(Clone, Debug)]
pub enum InputAction {
    CreatePreset,
    RenamePreset,
    MoveModToPosition(usize),
}

#[derive(Clone, Debug)]
pub enum ChecklistAction {
    ExportPresets,
    ImportPresets(PresetImportData),
    Backup,
    Restore(String), // filename
}

#[derive(Clone, Debug)]
pub enum MissingModsAction {
    ModImport(Vec<ModEntry>),
    PresetImport(PresetImportData),
}

#[derive(Clone, Debug)]
pub struct ChecklistItem {
    pub id: String,
    pub label: String,
    pub checked: bool,
}

#[derive(Clone, Debug)]
pub struct PresetImportData {
    pub presets: BTreeMap<String, Vec<ModEntry>>,
    pub selected_names: Vec<String>,
}

// ── Drag State ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct DragState {
    /// Index of the item being dragged in `current_mods`. Does NOT change during the drag
    /// (no live reordering). Committed only on mouse release.
    pub source_index: usize,
    pub pre_drag_snapshot: Vec<ModEntry>,
}

// ── Feature State Structs ──────────────────────────────────────────────────

/// BUG-2 FIX: Single authoritative state for Save Monitor.
/// All mutation checks use `is_running()`.
pub struct SaveMonitorState {
    pub running: bool,
    pub last_snapshot: Option<Instant>,
    pub snapshot_count: u32,
}

impl SaveMonitorState {
    pub fn new() -> Self {
        Self {
            running: false,
            last_snapshot: None,
            snapshot_count: 0,
        }
    }

    /// Single source of truth for whether mutations should be blocked.
    pub fn is_running(&self) -> bool {
        self.running
    }
}

pub struct BackupState {
    pub in_progress: bool,
    pub restoring: bool,
    pub backup_list: Vec<BackupInfo>,
    pub snapshot_list: Vec<MonitorSnapshot>,
    pub workshop_status: Vec<(String, bool)>,
    pub auto_backup_due: Option<Instant>,
}

impl BackupState {
    pub fn new() -> Self {
        Self {
            in_progress: false,
            restoring: false,
            backup_list: Vec::new(),
            snapshot_list: Vec::new(),
            workshop_status: Vec::new(),
            auto_backup_due: None,
        }
    }
}

pub struct GalleryState {
    pub catalog: Option<Catalog>,
    pub catalog_fetched_at: Option<Instant>,
    pub search_query: String,
    pub selected_tags: Vec<String>,
    pub loading: bool,
    pub error: Option<String>,
}

impl GalleryState {
    pub fn new() -> Self {
        Self {
            catalog: None,
            catalog_fetched_at: None,
            search_query: String::new(),
            selected_tags: Vec::new(),
            loading: false,
            error: None,
        }
    }
}

pub struct FileWatcherState {
    pub last_check: Option<Instant>,
    pub last_modified_time: u64,
    pub check_interval: std::time::Duration,
}

impl FileWatcherState {
    pub fn new() -> Self {
        Self {
            last_check: None,
            last_modified_time: 0,
            check_interval: std::time::Duration::from_secs(5),
        }
    }
}

// ── Preset Export/Import Format ────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PresetExportFile {
    pub hallinta_export: String,
    pub version: String,
    pub presets: BTreeMap<String, Vec<ModEntry>>,
    #[serde(default)]
    pub checksum: Option<String>,
}

// ── Mod List Export/Import Format ──────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModListEntry {
    pub name: String,
    #[serde(default, alias = "workshopId")]
    pub workshop_id: String,
}
