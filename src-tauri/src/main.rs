use named_lock::NamedLock;
use std::process;
use std::thread::sleep;
use std::time::Duration;

fn get_parent_process_name() -> Option<String> {
    use sysinfo::{Pid, Process, System};

    let current_pid = sysinfo::get_current_pid().ok()?;
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    if let Some(process) = sys.process(current_pid) {
        if let Some(parent_pid) = process.parent() {
            sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[parent_pid]), true);
            if let Some(parent_process) = sys.process(parent_pid) {
                return parent_process.name().to_str().map(|s| s.to_lowercase());
            }
        }
    }

    None
}

fn main() {
    // Check parent process and hide console on Windows only
    #[cfg(windows)]
    {
        let parent_process = get_parent_process_name();
        let run_without_console = matches!(&parent_process, Some(s) if s == "explorer.exe");

        if run_without_console {
            unsafe {
                winapi::um::wincon::FreeConsole();
            }
        }
    }

    // Existing NamedLock logic
    let lock_name = "hallinta_noita";
    let lock_result = NamedLock::create(lock_name);

    match lock_result {
        Ok(lock) => {
            match lock.try_lock() {
                Ok(_guard) => {
                    // Lock acquired successfully, proceed with the program
                    hallinta_lib::run();
                    // Simulate some work
                    sleep(Duration::from_secs(10));
                    // The guard will release the lock when it goes out of scope
                }
                Err(_) => {
                    // Lock is already held by another instance
                    eprintln!("Another instance of the program is already running.");
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            // Failed to create the lock
            eprintln!("Failed to create lock: {}", e);
            process::exit(1);
        }
    }
}