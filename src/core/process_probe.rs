//! Game process presence by image name only.
//! Paths and PIDs are intentionally ignored: Noita/EW install locations vary.

use crate::models::AutoMonitorWhen;

/// Whether known Noita / Entangled Worlds proxy processes appear to be running.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameProcessPresence {
    pub noita: bool,
    pub entangled_proxy: bool,
}

impl GameProcessPresence {
    pub fn matches(self, when: AutoMonitorWhen) -> bool {
        match when {
            AutoMonitorWhen::Noita => self.noita,
            AutoMonitorWhen::NoitaAndEntangled => self.noita && self.entangled_proxy,
        }
    }

    pub fn describe(self, when: AutoMonitorWhen) -> String {
        match when {
            AutoMonitorWhen::Noita if self.noita => "Noita is running.".to_string(),
            AutoMonitorWhen::NoitaAndEntangled if self.noita && self.entangled_proxy => {
                "Noita and the Entangled Worlds proxy are running.".to_string()
            }
            _ => "A watched game process is running.".to_string(),
        }
    }
}

/// Classify process image names (e.g. `noita.exe`, `noita_proxy`).
pub fn presence_from_image_names<I, S>(names: I) -> GameProcessPresence
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut presence = GameProcessPresence::default();
    for name in names {
        let normalized = normalize_image_name(name.as_ref());
        match normalized.as_str() {
            "noita" | "noita_dev" => presence.noita = true,
            "noita_proxy" => presence.entangled_proxy = true,
            _ => {}
        }
    }
    presence
}

fn normalize_image_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let without_exe = base
        .strip_suffix(".exe")
        .or_else(|| base.strip_suffix(".EXE"))
        .unwrap_or(base);
    without_exe.to_ascii_lowercase()
}

/// Snapshot currently running process image names and classify them.
pub fn detect_game_processes() -> GameProcessPresence {
    presence_from_image_names(running_image_names())
}

#[cfg(windows)]
fn running_image_names() -> Vec<String> {
    windows_image_names().unwrap_or_default()
}

#[cfg(not(windows))]
fn running_image_names() -> Vec<String> {
    Vec::new()
}

#[cfg(windows)]
fn windows_image_names() -> Result<Vec<String>, ()> {
    use std::mem::{size_of, zeroed};

    const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
    const INVALID_HANDLE_VALUE: isize = -1;
    const MAX_PATH: usize = 260;

    #[repr(C)]
    struct ProcessEntry32W {
        dw_size: u32,
        cnt_usage: u32,
        th32_process_id: u32,
        th32_default_heap_id: usize,
        th32_module_id: u32,
        cnt_threads: u32,
        th32_parent_process_id: u32,
        pc_pri_class_base: i32,
        dw_flags: u32,
        sz_exe_file: [u16; MAX_PATH],
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> isize;
        fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
        fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == 0 || snapshot == INVALID_HANDLE_VALUE {
            return Err(());
        }

        let mut entry: ProcessEntry32W = zeroed();
        entry.dw_size = size_of::<ProcessEntry32W>() as u32;
        let mut names = Vec::new();

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let len = entry
                    .sz_exe_file
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.sz_exe_file.len());
                if len > 0 {
                    names.push(String::from_utf16_lossy(&entry.sz_exe_file[..len]));
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AutoMonitorWhen;

    #[test]
    fn classifies_noita_variants_and_proxy_by_image_name() {
        let presence = presence_from_image_names([
            "NOITA.EXE",
            r"C:\Games\noita_proxy.exe",
            "chrome.exe",
            "noita_dev",
        ]);
        assert!(presence.noita);
        assert!(presence.entangled_proxy);
        assert!(presence.matches(AutoMonitorWhen::Noita));
        assert!(presence.matches(AutoMonitorWhen::NoitaAndEntangled));
    }

    #[test]
    fn noita_only_trigger_does_not_require_proxy() {
        let presence = presence_from_image_names(["noita.exe"]);
        assert!(presence.matches(AutoMonitorWhen::Noita));
        assert!(!presence.matches(AutoMonitorWhen::NoitaAndEntangled));
    }

    #[test]
    fn proxy_alone_never_matches() {
        let presence = presence_from_image_names(["noita_proxy.exe"]);
        assert!(!presence.matches(AutoMonitorWhen::Noita));
        assert!(!presence.matches(AutoMonitorWhen::NoitaAndEntangled));
    }
}
