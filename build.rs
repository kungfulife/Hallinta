use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Libraries we want to attribute (crate name -> (purpose, homepage))
const ATTRIBUTED_LIBS: &[(&str, &str, &str)] = &[
    ("eframe", "GUI application framework", "https://crates.io/crates/eframe"),
    ("egui", "Immediate-mode GUI library", "https://crates.io/crates/egui"),
    ("egui_extras", "Additional egui widgets", "https://crates.io/crates/egui_extras"),
    ("rfd", "Native file dialogs", "https://crates.io/crates/rfd"),
    ("opener", "Open files/URLs with OS handler", "https://crates.io/crates/opener"),
    ("image", "Image loading", "https://crates.io/crates/image"),
    ("serde", "Serialization framework", "https://crates.io/crates/serde"),
    ("serde_json", "JSON serialization", "https://crates.io/crates/serde_json"),
    ("dirs", "Platform directory lookup", "https://crates.io/crates/dirs"),
    ("tokio", "Async runtime", "https://crates.io/crates/tokio"),
    ("chrono", "Date/time handling", "https://crates.io/crates/chrono"),
    ("zip", "ZIP archive support", "https://crates.io/crates/zip"),
    ("named-lock", "Single-instance process lock", "https://crates.io/crates/named-lock"),
    ("walkdir", "Recursive directory traversal", "https://crates.io/crates/walkdir"),
    ("reqwest", "HTTP client", "https://crates.io/crates/reqwest"),
    ("sha2", "SHA-256 checksums", "https://crates.io/crates/sha2"),
];

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

/// Parse Cargo.lock to extract package versions.
fn parse_cargo_lock_versions() -> HashMap<String, String> {
    let mut versions = HashMap::new();
    let content = match std::fs::read_to_string("Cargo.lock") {
        Ok(c) => c,
        Err(_) => return versions,
    };

    let mut current_name: Option<String> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            current_name = None;
        } else if let Some(name) = trimmed.strip_prefix("name = \"") {
            current_name = name.strip_suffix('"').map(|s| s.to_string());
        } else if let Some(ver) = trimmed.strip_prefix("version = \"") {
            if let (Some(name), Some(ver)) = (current_name.take(), ver.strip_suffix('"')) {
                versions.entry(name).or_insert_with(|| ver.to_string());
            }
        }
    }
    versions
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

    // Generate open-source library list with actual versions from Cargo.lock (BUG-4 fix)
    let lock_versions = parse_cargo_lock_versions();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("libraries.rs");
    let mut f = std::fs::File::create(dest_path).unwrap();

    writeln!(f, "pub fn generated_open_source_libraries() -> Vec<crate::models::OpenSourceLibrary> {{").unwrap();
    writeln!(f, "    vec![").unwrap();
    for (name, purpose, homepage) in ATTRIBUTED_LIBS {
        let version = lock_versions.get(*name).cloned().unwrap_or_else(|| "?".to_string());
        writeln!(
            f,
            "        crate::models::OpenSourceLibrary {{ name: \"{name}\".into(), version: \"{version}\".into(), purpose: \"{purpose}\".into(), homepage: \"{homepage}\".into() }},",
        ).unwrap();
    }
    writeln!(f, "    ]").unwrap();
    writeln!(f, "}}").unwrap();

    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=build.rs");
}
