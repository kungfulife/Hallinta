use crate::models::MonitorResume;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestartIntent {
    pub monitor_resume: Option<MonitorResume>,
}

pub type RestartRequest = Arc<Mutex<Option<RestartIntent>>>;

pub fn new_request() -> RestartRequest {
    Arc::new(Mutex::new(None))
}

pub fn parse_monitor_resume(args: &[OsString]) -> Result<Option<MonitorResume>, String> {
    if args.is_empty() {
        return Ok(None);
    }
    if args.len() != 3 || args[0] != "--resume-monitor" {
        return Err("Invalid Hallinta restart arguments.".to_string());
    }
    let preset_name = args[1].to_string_lossy().into_owned();
    let session_id = args[2].to_string_lossy().into_owned();
    if preset_name.is_empty() || session_id.is_empty() {
        return Err("Monitor restart arguments cannot be empty.".to_string());
    }
    Ok(Some(MonitorResume {
        preset_name,
        session_id,
    }))
}

fn launch_args(resume: Option<&MonitorResume>) -> Vec<OsString> {
    match resume {
        Some(resume) => vec![
            "--resume-monitor".into(),
            resume.preset_name.clone().into(),
            resume.session_id.clone().into(),
        ],
        None => Vec::new(),
    }
}

fn launch_command(executable: &Path, resume: Option<&MonitorResume>) -> Command {
    let mut command = Command::new(executable);
    command.args(launch_args(resume));
    command
}

pub fn launch_updated(executable: &Path, resume: Option<&MonitorResume>) -> Result<(), String> {
    launch_command(executable, resume)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Hallinta updated but could not restart automatically: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MonitorResume;
    use std::ffi::OsString;

    #[test]
    fn monitor_resume_arguments_round_trip() {
        let resume = MonitorResume {
            preset_name: "Daily Run".to_string(),
            session_id: "session-42".to_string(),
        };
        let args = launch_args(Some(&resume));
        assert_eq!(parse_monitor_resume(&args).unwrap(), Some(resume));
    }

    #[test]
    fn ordinary_start_has_no_monitor_resume() {
        assert_eq!(parse_monitor_resume(&[]).unwrap(), None);
    }

    #[test]
    fn restart_without_monitor_is_still_a_restart_request() {
        let request = new_request();
        *request.lock().unwrap() = Some(RestartIntent {
            monitor_resume: None,
        });
        assert!(request.lock().unwrap().take().is_some());
    }

    #[test]
    fn malformed_resume_is_rejected() {
        let args = vec![
            OsString::from("--resume-monitor"),
            OsString::from("Daily Run"),
        ];
        assert!(parse_monitor_resume(&args).is_err());
    }

    #[test]
    fn relaunch_targets_the_captured_install_path() {
        let executable = std::path::PathBuf::from(r"C:\Apps\Hallinta.exe");
        let command = launch_command(&executable, None);
        assert_eq!(command.get_program(), executable.as_os_str());
    }
}
