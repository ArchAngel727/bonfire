// src-tauri/src/lib.rs
//
// Merge these bits into your existing lib.rs. The invoke_handler list has
// changed since v1: crypto_generate_one_time_keys is gone (we now generate
// at init and expose a single crypto_get_bundle_for_upload command).

mod crypto;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("no app data dir");
            let state = crypto::CryptoState::load_or_init(data_dir)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crypto::crypto_get_identity,
            crypto::crypto_get_bundle_for_upload,
            crypto::crypto_mark_keys_published,
            crypto::crypto_establish_session,
            crypto::crypto_has_session,
            crypto::crypto_encrypt,
            crypto::crypto_decrypt,
            crypto::crypto_attach,
            crypto::crypto_detach,
            crypto::crypto_regenerate,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// --- Tauri 1.x note ---
// If you're on Tauri 1.x:
//   - drop the `use tauri::Manager;` line (it's in the prelude)
//   - replace `app.path().app_data_dir()` with
//     `tauri::api::path::app_data_dir(&app.config()).ok_or("no app data dir")?`
//   - the entry point is usually `fn main()` calling
//     `tauri::Builder::default()...` directly rather than `pub fn run()`.
