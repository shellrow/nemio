// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod define;
mod validator;
mod db;
mod db_models;
mod json_models;
mod arg_models;
mod commands;
use commands::{
  exec_portscan, 
  exec_hostscan, 
  exec_ping, 
  exec_traceroute, 
  lookup_hostname, 
  lookup_ipaddr, 
  get_probe_log, 
  get_probed_hosts, 
  save_map_data, 
  get_map_data, 
  get_top_probe_hist, 
  get_probe_stat,
  get_default_interface,
  get_port_scan_result,
  get_host_scan_result,
  get_ping_stat,
  get_trace_result,
  get_os_type,
  save_user_probe_data,
  save_user_group,
  save_user_tag,
  get_user_probe_data,
  get_all_user_probe_data,
  get_user_hosts,
  get_valid_user_hosts,
  enable_user_host,
  disable_user_host,
  delete_user_host,
  get_new_host_id,
  get_app_info
};

fn main() {
  // Check if we are running as root
  if !rushmap_core::process::privileged() {
    rushmap_core::process::restart_as_root(true);
  }
  // Run the Tauri application
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      exec_portscan, 
      exec_hostscan,
      exec_ping,
      exec_traceroute,
      lookup_hostname,
      lookup_ipaddr,
      get_probe_log,
      get_probed_hosts,
      save_map_data,
      get_map_data,
      get_top_probe_hist,
      get_probe_stat,
      get_default_interface,
      get_port_scan_result,
      get_host_scan_result,
      get_ping_stat,
      get_trace_result,
      get_os_type,
      save_user_probe_data,
      save_user_group,
      save_user_tag,
      get_user_probe_data,
      get_all_user_probe_data,
      get_user_hosts,
      get_valid_user_hosts,
      enable_user_host,
      disable_user_host,
      delete_user_host,
      get_new_host_id,
      get_app_info
      ])
      .setup(|app| {
        let app_handle = app.handle();
        db::init(app_handle);
        Ok(())
      })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}