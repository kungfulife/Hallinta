// Removes Terminal on release build
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use named_lock::NamedLock;
use std::process;
use std::thread::sleep;
use std::time::Duration;

fn main() {
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