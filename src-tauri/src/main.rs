#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod deeplink;

use tauri::Emitter;

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Fixes black rectangles, missing repaints, and transition glitch artifacts on Linux WebKitGTK
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // Inspect command-line arguments for any szz:// custom protocol URL
    let deep_link_arg = std::env::args().find(|a| a.starts_with("szz://"));

    if let Some(ref url) = deep_link_arg {
        // Attempt to forward URL to already running instance via single-instance socket
        let forwarded = tauri::async_runtime::block_on(async {
            deeplink::try_forward_to_existing_instance(url).await
        });
        if forwarded {
            // Forwarding succeeded; gracefully exit duplicate process
            return;
        }
    }

    tauri::Builder::default()
        .setup(move |app| {
            // Spawn single-instance IPC listener for incoming deep-links
            deeplink::spawn_single_instance_listener(app.handle().clone());

            // If this primary instance was launched with a deep-link argument,
            // emit the event once the webview UI is ready.
            if let Some(url) = deep_link_arg {
                if let Some(server_id) = deeplink::parse_deep_link_url(&url) {
                    let handle = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                        let _ = handle.emit("szz://deep-link", server_id);
                    });
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::launch_server,
            commands::launch_instance,
            commands::get_instance_status,
            commands::install_instance,
            commands::uninstall_instance,
            commands::list_installed_instances,
            commands::get_sample_manifest_path,
            commands::load_manifest,
            commands::get_auth_state,
            commands::start_login_loopback,
            commands::start_login_device_code,
            commands::poll_login_device_code,
            commands::switch_account,
            commands::remove_account,
            commands::create_dev_account,
            commands::open_external_url,
            commands::get_server_catalog,
            commands::update_instance,
            commands::get_storage_usage,
            commands::clean_storage_cache,
            commands::get_settings,
            commands::update_settings,
            commands::reset_settings,
            commands::get_system_memory_info,
            commands::ping_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
