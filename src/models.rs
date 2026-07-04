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
    pub save_monitor_settings: SaveMonitorSettings,
    #[serde(default)]
    pub steam_path: String,
    #[serde(default)]
    pub compact_mode: bool,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    #[serde(default)]
    pub last_filter_mode: String,
    #[serde(default)]
    pub last_sort_mode: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogSettings {
    #[serde(default = "default_max_log_files")]
    pub max_log_files: usize,
    #[serde(default = "default_max_log_size_mb")]
    pub max_log_size_mb: usize,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub collect_system_info: bool,
}

fn default_max_log_files() -> usize {
    50
}

fn default_max_log_size_mb() -> usize {
    10
}

fn default_log_level() -> String {
    "INFO".to_string()
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            max_log_files: default_max_log_files(),
            max_log_size_mb: default_max_log_size_mb(),
            log_level: default_log_level(),
            collect_system_info: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveMonitorSettings {
    #[serde(default = "default_max_snapshots_per_session")]
    #[serde(alias = "max_snapshots_per_preset")]
    pub max_snapshots_per_session: usize,
    #[serde(default = "default_backup_delay_minutes")]
    #[serde(alias = "interval_minutes")]
    pub backup_delay_minutes: u64,
    #[serde(default)]
    pub include_entangled: bool,
    #[serde(default = "default_include_save01")]
    pub include_save01: bool,
    #[serde(default)]
    pub start_in_monitor_mode: bool,
}

fn default_max_snapshots_per_session() -> usize {
    15
}

fn default_backup_delay_minutes() -> u64 {
    3
}

fn default_include_save01() -> bool {
    true
}

fn default_ui_scale() -> f32 {
    1.25
}

impl Default for SaveMonitorSettings {
    fn default() -> Self {
        Self {
            max_snapshots_per_session: 15,
            backup_delay_minutes: 3,
            include_entangled: false,
            include_save01: true,
            start_in_monitor_mode: false,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkshopInstallState {
    Installed,
    Missing,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopCheckReport {
    pub statuses: Vec<(String, WorkshopInstallState)>,
    pub libraries_checked: Vec<String>,
    pub content_roots_found: usize,
    pub diagnostic: Option<String>,
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
    pub git_hash: String,
    pub build_profile: String,
    pub rust_version: String,
    pub cargo_version: String,
    pub build_target: String,
    pub gui_framework: String,
    pub os: String,
    pub os_family: String,
    pub arch: String,
    #[serde(default)]
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

// ── Session System ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SessionStatus {
    #[serde(alias = "Active")]
    Monitoring,
    #[serde(alias = "Ended")]
    Paused,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub preset_name: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: SessionStatus,
    pub snapshot_count: u32,
    pub locked_mods: Vec<ModEntry>,
    /// On-disk folder name under the preset directory. Defaults to `id` when empty.
    #[serde(default)]
    pub folder_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SnapshotEntry {
    pub filename: String,
    pub session_id: String,
    pub timestamp: String,
    pub size_bytes: u64,
}

// ── UI State Enums ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    ModList,
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
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "enabled" => Self::Enabled,
            "disabled" => Self::Disabled,
            _ => Self::All,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortMode {
    Default,
    NameAsc,
    NameDesc,
    EnabledFirst,
    DisabledFirst,
}

impl SortMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Order",
            Self::NameAsc => "A → Z",
            Self::NameDesc => "Z → A",
            Self::EnabledFirst => "Enabled ↑",
            Self::DisabledFirst => "Disabled ↑",
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::NameAsc => "name_asc",
            Self::NameDesc => "name_desc",
            Self::EnabledFirst => "enabled_first",
            Self::DisabledFirst => "disabled_first",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "name_asc" => Self::NameAsc,
            "name_desc" => Self::NameDesc,
            "enabled_first" => Self::EnabledFirst,
            "disabled_first" => Self::DisabledFirst,
            _ => Self::Default,
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
        dismissable: bool,
    },
    Input {
        title: String,
        value: String,
        hint: String,
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
    ExternalModChanges {
        file_mods: Vec<ModEntry>,
        summary: ExternalModChangeSummary,
    },
    SystemInfo,
    OpenSourceLibraries,
    BackupManager,
    RestoreManager {
        sessions: Vec<SessionInfo>,
        snapshots: Vec<SnapshotEntry>,
        /// Which view: None = session list, Some(session_id) = snapshot list for that session
        selected_session: Option<(String, String)>, // (session_id, session_name)
    },
}

#[derive(Clone, Debug)]
pub enum ConfirmAction {
    DeletePreset,
    /// (preferred_index, name, workshop_id) — index is a hint, name+workshop_id used to verify.
    DeleteMod(usize, String, String),
    AcceptExternalChanges(Vec<ModEntry>),
    KeepCurrentPreset,
    OverwritePresetImport(PresetImportData),
    RenamePresetImport(PresetImportData),
    ChecksumMismatchContinue(PresetImportData),
    ExitWithSnapshot,
    ExitWithoutSnapshot,
    DeleteBackup(String),
    DeleteMonitorSession {
        preset_name: String,
        session_id: String,
        session_name: String,
    },
    RestoreLatest(String),
    ClearMonitorData,
    ContinueMonitorSession(String), // session_id
    StartNewMonitorSession,
    DismissConfirm,
}

#[derive(Clone, Debug)]
pub enum InputAction {
    CreatePreset,
    RenamePreset,
    MoveModToPosition(usize),
    StartMonitorSession,
    RenameMonitorSession {
        preset_name: String,
        session_id: String,
    },
}

#[derive(Clone, Debug)]
pub enum ChecklistAction {
    ExportPresets,
    ImportPresets(PresetImportData),
    Backup,
    Restore(String), // filename
    RestoreSnapshot(std::path::PathBuf),
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
    pub required: bool,
}

#[derive(Clone, Debug)]
pub struct PresetImportData {
    pub presets: BTreeMap<String, Vec<ModEntry>>,
    pub selected_names: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalModChangeSummary {
    pub current_total: usize,
    pub disk_total: usize,
    pub current_enabled: usize,
    pub disk_enabled: usize,
    pub added: usize,
    pub removed: usize,
    pub enabled_changed: usize,
    pub order_changed: bool,
}

// ── Drag State ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct DragState {
    /// Where the dragged item currently sits in `current_mods` — updated live as the
    /// pointer moves so the list previews the final order in real time.
    pub current_index: usize,
    pub pre_drag_snapshot: Vec<ModEntry>,
}

// ── Feature State Structs ──────────────────────────────────────────────────

/// BUG-2 FIX: Single authoritative state for Save Monitor.
/// All mutation checks use `is_running()`.
pub struct SaveMonitorState {
    pub running: bool,
    pub current_session: Option<SessionInfo>,
    pub snapshot_count: u32,
    pub last_known_mtime: u64,
    pub pending_change_since: Option<Instant>,
    pub snapshot_in_flight: bool,
    pub last_scan: Option<Instant>,
}

impl SaveMonitorState {
    pub fn new() -> Self {
        Self {
            running: false,
            current_session: None,
            snapshot_count: 0,
            last_known_mtime: 0,
            pending_change_since: None,
            snapshot_in_flight: false,
            last_scan: None,
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
    pub snapshot_list: Vec<SnapshotEntry>,
    pub workshop_status: Vec<(String, WorkshopInstallState)>,
    pub workshop_check_generation: u64,
    pub workshop_check_in_flight: bool,
    pub workshop_diagnostic: Option<String>,
}

impl BackupState {
    pub fn new() -> Self {
        Self {
            in_progress: false,
            restoring: false,
            backup_list: Vec::new(),
            snapshot_list: Vec::new(),
            workshop_status: Vec::new(),
            workshop_check_generation: 0,
            workshop_check_in_flight: false,
            workshop_diagnostic: None,
        }
    }
}

pub struct FileWatcherState {
    pub last_check: Option<Instant>,
    pub last_modified_time: u64,
    pub check_interval: std::time::Duration,
    pub pending_external_mods: Option<Vec<ModEntry>>,
}

impl FileWatcherState {
    pub fn new() -> Self {
        Self {
            last_check: None,
            last_modified_time: 0,
            check_interval: std::time::Duration::from_secs(5),
            pending_external_mods: None,
        }
    }
}

#[cfg(test)]
mod log_settings_tests {
    use super::{LogSettings, SaveMonitorSettings};

    #[test]
    fn log_settings_default_has_info_level() {
        assert_eq!(LogSettings::default().log_level, "INFO");
    }

    #[test]
    fn save_monitor_default_backup_delay_is_three_minutes() {
        assert_eq!(SaveMonitorSettings::default().backup_delay_minutes, 3);
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
