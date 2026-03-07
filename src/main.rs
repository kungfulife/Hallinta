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
            eprintln!("Failed to create lock: {}", e);
            process::exit(1);
        }
    };

    let _guard = match lock.try_lock() {
        Ok(g) => g,
        Err(_) => {
            eprintln!("Another instance of Hallinta is already running.");
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

    // Step 7: Configure eframe window
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_min_inner_size([1050.0, 800.0])
            .with_icon(icon_data),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    // Step 8: Run the application
    let title = core::platform::get_window_title();
    let result = eframe::run_native(
        &title,
        options,
        Box::new(move |cc| Ok(Box::new(app::HallintaApp::new(cc, rt_handle)))),
    );

    if let Err(e) = result {
        eprintln!("Application error: {}", e);
    }

    // Lock guard drops here immediately — no sleep (BUG-3 fix)
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
