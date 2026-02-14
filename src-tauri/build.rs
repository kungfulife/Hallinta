use std::process::Command;

fn command_output(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn main() {
    let rustc_version = command_output("rustc", &["--version"]);
    let cargo_version = command_output("cargo", &["--version"]);
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=HALLINTA_RUSTC_VERSION={rustc_version}");
    println!("cargo:rustc-env=HALLINTA_CARGO_VERSION={cargo_version}");
    println!("cargo:rustc-env=HALLINTA_TARGET={target}");
    println!("cargo:rustc-env=HALLINTA_PROFILE={profile}");

    tauri_build::build()
}
