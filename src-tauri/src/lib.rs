use serde::Serialize;

mod cs2;
mod display;
mod mouse;
mod nvidia;
mod pubg;
mod release;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformInfo {
    os: String,
    version: String,
    architecture: String,
    desktop_runtime: bool,
}

#[tauri::command]
fn get_platform_info() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        version: std::env::var("OS").unwrap_or_else(|_| "Unknown".to_string()),
        architecture: std::env::consts::ARCH.to_string(),
        desktop_runtime: true,
    }
}

#[tauri::command]
fn capture_cs2_settings() -> cs2::Cs2CommandResponse {
    cs2::capture()
}

#[tauri::command]
fn apply_cs2_settings(payload: cs2::Cs2Payload) -> cs2::Cs2CommandResponse {
    cs2::apply(payload)
}

#[tauri::command]
fn check_cs2_closed() -> cs2::Cs2CommandResponse {
    cs2::preflight()
}

#[tauri::command]
fn restore_cs2_settings() -> cs2::Cs2CommandResponse {
    cs2::restore()
}

#[tauri::command]
fn capture_display_settings(
    request: display::DisplayCaptureRequest,
) -> display::DisplayCommandResponse {
    display::capture(request)
}

#[tauri::command]
fn apply_display_settings(payload: display::DisplayPayload) -> display::DisplayCommandResponse {
    display::apply(payload)
}

#[tauri::command]
fn get_display_diagnostics() -> display::DisplayCommandResponse {
    display::diagnostics()
}

#[tauri::command]
fn restore_display_settings(backup_token: Option<String>) -> display::DisplayCommandResponse {
    display::restore(backup_token)
}

#[tauri::command]
fn capture_nvidia_settings(request: nvidia::NvidiaCaptureRequest) -> nvidia::NvidiaCommandResponse {
    nvidia::capture(request)
}

#[tauri::command]
fn apply_nvidia_settings(payload: nvidia::NvidiaPayload) -> nvidia::NvidiaCommandResponse {
    nvidia::apply(payload)
}

#[tauri::command]
fn get_nvidia_diagnostics() -> nvidia::NvidiaCommandResponse {
    nvidia::diagnostics()
}

#[tauri::command]
fn restore_nvidia_settings(backup_token: Option<String>) -> nvidia::NvidiaCommandResponse {
    nvidia::restore(backup_token)
}

#[tauri::command]
fn capture_mouse_settings() -> mouse::MouseCommandResponse {
    mouse::capture()
}

#[tauri::command]
fn apply_mouse_settings(payload: mouse::MousePayload) -> mouse::MouseCommandResponse {
    mouse::apply(payload)
}

#[tauri::command]
fn get_mouse_diagnostics() -> mouse::MouseCommandResponse {
    mouse::diagnostics()
}

#[tauri::command]
fn restore_mouse_settings(backup_token: Option<String>) -> mouse::MouseCommandResponse {
    mouse::restore(backup_token)
}

#[tauri::command]
fn capture_pubg_settings() -> pubg::PubgCommandResponse {
    pubg::capture()
}

#[tauri::command]
fn apply_pubg_settings(payload: pubg::PubgPayload) -> pubg::PubgCommandResponse {
    pubg::apply(payload)
}

#[tauri::command]
fn check_pubg_closed() -> pubg::PubgCommandResponse {
    pubg::preflight()
}

#[tauri::command]
fn restore_pubg_settings(backup_token: Option<String>) -> pubg::PubgCommandResponse {
    pubg::restore(backup_token)
}

#[tauri::command]
fn get_pubg_diagnostics() -> pubg::PubgCommandResponse {
    pubg::diagnostics()
}

#[tauri::command]
fn get_release_preflight() -> release::ReleasePreflight {
    release::preflight()
}

#[tauri::command]
fn append_production_log(entry: release::ProductionLogEntry) -> release::FileCommandResponse {
    release::append_log(entry)
}

#[tauri::command]
fn save_diagnostic_report(contents: String) -> release::FileCommandResponse {
    release::save_report(contents)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_platform_info,
            check_cs2_closed,
            capture_cs2_settings,
            apply_cs2_settings,
            restore_cs2_settings,
            capture_display_settings,
            apply_display_settings,
            get_display_diagnostics,
            restore_display_settings,
            capture_nvidia_settings,
            apply_nvidia_settings,
            get_nvidia_diagnostics,
            restore_nvidia_settings,
            capture_mouse_settings,
            apply_mouse_settings,
            get_mouse_diagnostics,
            restore_mouse_settings,
            check_pubg_closed,
            capture_pubg_settings,
            apply_pubg_settings,
            restore_pubg_settings,
            get_pubg_diagnostics,
            get_release_preflight,
            append_production_log,
            save_diagnostic_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running Game Passport");
}
