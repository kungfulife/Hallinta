// Removes terminal on release build
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod core;
mod models;
mod tasks;
mod ui;

use named_lock::NamedLock;
use std::process;

fn main() {
    let startup_args: Vec<_> = std::env::args_os().skip(1).collect();
    let monitor_resume = match core::relaunch::parse_monitor_resume(&startup_args) {
        Ok(resume) => resume,
        Err(error) => {
            eprintln!("Hallinta startup failed: {error}");
            process::exit(1);
        }
    };
    // self-replace renames the running image on Windows. Capture the canonical
    // install path before any update so restart cannot target the relocated old binary.
    let installed_executable = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .unwrap_or_else(|error| {
            eprintln!("Hallinta startup failed: could not locate the executable: {error}");
            process::exit(1);
        });
    let restart_request = core::relaunch::new_request();
    let app_restart_request = restart_request.clone();
    // Step 1: Install panic hook
    core::logging::install_panic_logging_hook();

    // Step 2: Write session begin marker
    core::logging::init_log_session();

    // Step 3: Single-instance lock
    // BUG-3 FIX: Scope the lock guard to only the eframe::run_native() call.
    // When run_native returns, the guard drops immediately — no artificial sleep.
    let lock_name = "hallinta_noita";
    let lock = match NamedLock::create(lock_name) {
        Ok(l) => l,
        Err(e) => {
            let _ = core::logging::log("ERROR", &format!("Failed to create lock: {}", e), "Main");
            process::exit(1);
        }
    };

    let _guard = match lock.try_lock() {
        Ok(g) => g,
        Err(_) => {
            let _ = core::logging::log(
                "WARN",
                "Another instance of Hallinta is already running.",
                "Main",
            );
            process::exit(1);
        }
    };

    // Step 4: Initialize tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");
    let rt_handle = rt.handle().clone();

    // Step 5-6: Load icon
    let icon_data = load_icon();

    // Step 7: Configure eframe window — scale-aware initial sizing + multi-monitor centering
    let saved_settings = core::settings::load_settings().ok();
    if let Some(settings) = saved_settings.as_ref() {
        core::logging::configure(&settings.log_settings);
    }
    let saved_scale = saved_settings
        .as_ref()
        .map(|s| s.ui_scale)
        .unwrap_or(ui::design::SCALE_INTERNAL_DEFAULT);
    let compact = saved_settings.as_ref().is_some_and(|s| s.compact_mode);

    let (base_size, base_min) = if compact {
        (ui::design::BASE_SIZE_COMPACT, ui::design::BASE_MIN_COMPACT)
    } else {
        (ui::design::BASE_SIZE_NORMAL, ui::design::BASE_MIN_NORMAL)
    };
    let init_size = ui::design::scaled_size(base_size, saved_scale);
    let init_min = ui::design::scaled_min_size(base_min, saved_scale);

    // Center on the monitor where the cursor is (multi-monitor aware)
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(init_size)
        .with_min_inner_size(init_min)
        .with_icon(icon_data);

    let use_eframe_centered;
    if let Some((x, y)) = core::platform::get_cursor_monitor_center(init_size.x, init_size.y) {
        viewport = viewport.with_position(egui::pos2(x, y));
        use_eframe_centered = false;
    } else {
        // Fallback: let eframe center on primary monitor
        use_eframe_centered = true;
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: use_eframe_centered,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    // Step 8: Run the application
    let title = core::platform::get_window_title();
    let result = eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::HallintaApp::new(
                cc,
                rt_handle,
                monitor_resume,
                app_restart_request,
            )))
        }),
    );

    if let Err(e) = result {
        let _ = core::logging::log("ERROR", &format!("Application error: {}", e), "Main");
    }

    // Relaunch only after the single-instance lock is released.
    drop(_guard);
    let requested_restart = restart_request
        .lock()
        .ok()
        .and_then(|mut request| request.take());
    if let Some(intent) = requested_restart
        && let Err(error) =
            core::relaunch::launch_updated(&installed_executable, intent.monitor_resume.as_ref())
    {
        let _ = core::logging::log("ERROR", &error, "Updater");
    }
}

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("./assets/app-icon-32.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon")
        .to_rgba8();
    let (w, h) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width: w,
        height: h,
    }
}

use eframe::egui;
