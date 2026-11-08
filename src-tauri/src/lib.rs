mod commands;
mod credentials;
pub mod db;
pub mod engines;
pub mod error;
mod export;
pub mod models;
mod pg_error;
pub mod sql_split;
mod ssh_tunnel;

use commands::{
  apply_table_properties, cancel_veloxy_request, chat_with_db, check_for_updates, clear_veloxy_conversation,
  connect_db, delete_connection,
  delete_openrouter_api_key, disconnect_db, duckdb_get_schema, duckdb_get_tables, duckdb_run_query,
  execute_ddl_statement, execute_ddl_transaction, export_diagram_png,
  export_results_csv_command, export_results_json_command, generate_sql_from_nl, get_foreign_keys,
  get_openrouter_api_key, get_query_editor_metadata, get_schema, get_table_indexes, get_table_properties, get_tables,
  lint_sql, list_connections_command, list_databases, load_veloxy_conversation, mongo_get_collections,
  mongo_get_schema, mongo_run_query, ping_connection, redis_get_keys, redis_get_schema, redis_run_query,
  refresh_connection, rename_connection, run_query, save_base64_png, save_text_file, set_active_connection,
  store_openrouter_api_key, switch_database,
};
use db::AppState;
use tauri::Manager;

#[cfg(target_os = "android")]
unsafe extern "C" fn velox_native_signal_handler(
  sig: libc::c_int,
  info: *mut libc::siginfo_t,
  _ucontext: *mut libc::c_void,
) {
  let sig_name = match sig {
    libc::SIGSEGV => "SIGSEGV (Segmentation Fault)",
    libc::SIGBUS => "SIGBUS (Bus Error / Memory Alignment)",
    libc::SIGILL => "SIGILL (Illegal Instruction / Unsupported CPU Opcode)",
    libc::SIGABRT => "SIGABRT (Abort)",
    libc::SIGFPE => "SIGFPE (Floating Point Exception)",
    _ => "Unknown Signal",
  };

  let fault_addr = if !info.is_null() {
    format!("{:p}", (*info).si_addr())
  } else {
    "unknown".to_string()
  };

  let msg = format!(
    "FATAL NATIVE SIGNAL [{}]: signal {} at address {}\n",
    sig_name, sig, fault_addr
  );

  // 1. Android logcat output via __android_log_print with tag "VeloxNative"
  if let Ok(tag) = std::ffi::CString::new("VeloxNative") {
    if let Ok(c_msg) = std::ffi::CString::new(msg.clone()) {
      extern "C" {
        fn __android_log_print(
          prio: std::os::raw::c_int,
          tag: *const std::os::raw::c_char,
          fmt: *const std::os::raw::c_char,
          ...
        ) -> std::os::raw::c_int;
      }
      __android_log_print(6, tag.as_ptr(), c_msg.as_ptr());
    }
  }

  // 2. Direct POSIX file write to Download directory
  let path = b"/storage/emulated/0/Download/veloxdb_crash.txt\0";
  let fd = libc::open(
    path.as_ptr() as *const libc::c_char,
    libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
    0o666,
  );
  if fd >= 0 {
    let full_text = format!("================ VELOXDB NATIVE CRASH ================\n{}\n", msg);
    libc::write(fd, full_text.as_ptr() as *const libc::c_void, full_text.len());
    libc::close(fd);
  }

  // 3. Reset and re-raise default signal so Android debuggerd creates tombstone
  libc::signal(sig, libc::SIG_DFL);
  libc::raise(sig);
}

#[cfg(target_os = "android")]
fn install_native_signal_handlers() {
  unsafe {
    let mut sa: libc::sigaction = std::mem::zeroed();
    sa.sa_sigaction = velox_native_signal_handler as usize;
    sa.sa_flags = libc::SA_SIGINFO | libc::SA_RESETHAND;
    libc::sigemptyset(&mut sa.sa_mask);

    libc::sigaction(libc::SIGSEGV, &sa, std::ptr::null_mut());
    libc::sigaction(libc::SIGBUS, &sa, std::ptr::null_mut());
    libc::sigaction(libc::SIGILL, &sa, std::ptr::null_mut());
    libc::sigaction(libc::SIGABRT, &sa, std::ptr::null_mut());
    libc::sigaction(libc::SIGFPE, &sa, std::ptr::null_mut());
  }
}

#[no_mangle]
#[cfg(target_os = "android")]
pub extern "C" fn JNI_OnLoad(_vm: *mut std::ffi::c_void, _reserved: *mut std::ffi::c_void) -> i32 {
  install_native_signal_handlers();
  0x00010006 // JNI_VERSION_1_6
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  #[cfg(target_os = "android")]
  {
    install_native_signal_handlers();
    std::panic::set_hook(Box::new(|info| {
      let msg = format!("Rust PANIC: {}", info);
      if let Ok(tag) = std::ffi::CString::new("VeloxNative") {
        if let Ok(msg_c) = std::ffi::CString::new(msg.as_str()) {
          unsafe {
            extern "C" {
              fn __android_log_print(
                prio: std::os::raw::c_int,
                tag: *const std::os::raw::c_char,
                fmt: *const std::os::raw::c_char,
                ...
              ) -> std::os::raw::c_int;
            }
            __android_log_print(6, tag.as_ptr(), msg_c.as_ptr());
          }
        }
      }
      let path = b"/storage/emulated/0/Download/veloxdb_crash.txt\0";
      unsafe {
        let fd = libc::open(
          path.as_ptr() as *const libc::c_char,
          libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
          0o666,
        );
        if fd >= 0 {
          let full_text = format!("================ VELOXDB RUST PANIC ================\n{}\n", msg);
          libc::write(fd, full_text.as_ptr() as *const libc::c_void, full_text.len());
          libc::close(fd);
        }
      }
      eprintln!("{}", msg);
    }));
  }

  tauri::Builder::default()
    .manage(AppState::default())
    .plugin(tauri_plugin_store::Builder::default().build())
    .plugin(tauri_plugin_dialog::init())
    .setup(|app| {
      #[cfg(desktop)]
      {
        // Guard desktop window operations (decorations, window resizing, centering)
        if cfg!(debug_assertions) {
          app.handle().plugin(
            tauri_plugin_log::Builder::default()
              .level(log::LevelFilter::Info)
              .build(),
          )?;
        }
      }
      #[cfg(not(desktop))]
      {
        let _ = app;
      }
      Ok(())
    })
    .on_window_event(|window, event| {
      #[cfg(desktop)]
      if let tauri::WindowEvent::Destroyed = event {
        let handle = window.app_handle();
        let state: tauri::State<AppState> = handle.state();
        let mut tunnels = state.ssh_tunnels.blocking_write();
        for (_, mut tunnel) in tunnels.drain() {
          tauri::async_runtime::block_on(async {
            tunnel.close().await;
          });
        }
      }
      #[cfg(not(desktop))]
      {
        let _ = (window, event);
      }
    })
    .invoke_handler(tauri::generate_handler![
      check_for_updates,
      connect_db,
      disconnect_db,
      rename_connection,
      delete_connection,
      ping_connection,
      refresh_connection,
      list_connections_command,
      set_active_connection,
      list_databases,
      switch_database,
      run_query,
      get_tables,
      get_schema,
      get_table_properties,
      apply_table_properties,
      get_foreign_keys,
      get_table_indexes,
      execute_ddl_transaction,
      execute_ddl_statement,
      export_diagram_png,
      export_results_csv_command,
      export_results_json_command,
      get_query_editor_metadata,
      save_base64_png,
      save_text_file,
      lint_sql,
      generate_sql_from_nl,
      chat_with_db,
      cancel_veloxy_request,
      load_veloxy_conversation,
      clear_veloxy_conversation,
      store_openrouter_api_key,
      get_openrouter_api_key,
      delete_openrouter_api_key,
      mongo_run_query,
      mongo_get_collections,
      mongo_get_schema,
      duckdb_run_query,
      duckdb_get_tables,
      duckdb_get_schema,
      redis_run_query,
      redis_get_keys,
      redis_get_schema
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
