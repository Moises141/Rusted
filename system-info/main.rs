#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sysinfo::{CpuExt, System, SystemExt};
use wgpu::{Backends, Instance};
use tokio::time;

slint::include_modules!();

/// Asynchronously retrieves GPU information, including VRAM capacity and clock speed if available.
async fn get_gpu_info() -> String {
    let instance = Instance::default();
    let adapters = instance.enumerate_adapters(Backends::all());

    if adapters.is_empty() {
        return "No GPU adapters found".to_string();
    }

    let mut gpu_info = String::new();
    let mut seen_devices = HashSet::new();

    for adapter in adapters {
        let info = adapter.get_info();
        if !seen_devices.contains(&info.device) {
            seen_devices.insert(info.device);

            // Retrieve VRAM limits (approximation of available VRAM)
            let limits = adapter.limits();
            let vram_capacity_mb = limits.max_storage_buffer_binding_size / (1024 * 1024);

            gpu_info.push_str(&format!(
                "GPU: {} ({:?})\n",  // Simplified display of GPU name and backend
                info.name, info.backend
            ));
            gpu_info.push_str(&format!("VRAM: {} MB\n", vram_capacity_mb)); // VRAM in MB
            gpu_info.push_str("Clock Speed: N/A (requires vendor-specific APIs)\n"); // Placeholder
            break; // Only show information for the first GPU (if there are multiple)
        }
    }

    gpu_info.trim_end().to_string()
}


/// Updates system information in the UI.
fn update_system_info(ui: &AppWindow, system: &mut System, gpu_info: &str) {
    // Refresh system information
    system.refresh_all();

    // Update CPU usage
    let cpus = system.cpus();
    let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32;
    ui.set_cpu_usage(format!("CPU Usage: {:.2}%", total_usage).into());

    // Update RAM usage
    let total_memory = system.total_memory() / 1024 / 1024;
    let used_memory = system.used_memory() / 1024 / 1024;
    let free_memory = system.free_memory() / 1024 / 1024;
    ui.set_ram_info(
        format!(
            "Total RAM: {} MB, Used: {} MB, Free: {} MB",
            total_memory, used_memory, free_memory
        )
        .into(),
    );

    // Update GPU information
    ui.set_gpu_info(gpu_info.to_string().into());
}

#[tokio::main] // Use Tokio runtime for async support
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Initialize the UI and system monitor
    let ui = AppWindow::new()?;
    let system = Arc::new(Mutex::new(System::new_all()));

    // Fetch GPU information asynchronously
    let gpu_info = get_gpu_info().await;

    // Periodically update system information
    {
        let ui_handle = ui.as_weak();
        let system = Arc::clone(&system);
        let gpu_info = gpu_info.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if let Some(ui) = ui_handle.upgrade() {
                    if let Ok(mut system) = system.lock() {
                        update_system_info(&ui, &mut system, &gpu_info);
                    }
                }
            }
        });
    }

    // UI logic for button click handling (if applicable)
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        let system = Arc::clone(&system);
        let gpu_info = gpu_info.clone();

        move || {
            if let Some(ui) = ui_handle.upgrade() {
                if let Ok(mut system) = system.lock() {
                    update_system_info(&ui, &mut system, &gpu_info);
                }
            }
        }
    });

    // Run the UI
    ui.run()?;
    Ok(())
}
