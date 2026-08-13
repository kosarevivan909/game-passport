use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_SETTINGS: usize = 32;

const PORTABLE_SETTINGS: &[(u32, &str)] = &[
    (0x1057EB71, "power_management_mode"),
    (0x10835002, "max_frame_rate"),
    (0x00A879CF, "vertical_sync"),
    (0x00CE2691, "texture_filtering_quality"),
    (0x00198FFF, "shader_cache"),
    (0x00AC8497, "shader_cache_size"),
    (0x10D2BB16, "anisotropic_filtering_mode"),
    (0x101E61A9, "anisotropic_filtering_level"),
    (0x00E73211, "anisotropic_sample_optimization"),
    (0x0084CD70, "anisotropic_filter_optimization"),
    (0x002ECAF2, "trilinear_optimization"),
    (0x0019BB68, "negative_lod_bias"),
    (0x007BA09E, "maximum_pre_rendered_frames"),
    (0x1074C972, "fxaa"),
    (0x0098C1AC, "mfaa"),
    (0x0064B541, "preferred_refresh_rate"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaCaptureRequest {
    pub game: String,
}

#[derive(Clone, Copy)]
struct NvidiaTarget {
    executable: &'static str,
    profile_name: &'static str,
    friendly_name: &'static str,
}

fn target_for_game(game: &str) -> Result<NvidiaTarget, String> {
    match game {
        "cs2" => Ok(NvidiaTarget {
            executable: "cs2.exe",
            profile_name: "Game Passport - Counter-Strike 2",
            friendly_name: "Counter-Strike 2",
        }),
        "pubg" => Ok(NvidiaTarget {
            executable: "TslGame.exe",
            profile_name: "Game Passport - PUBG BATTLEGROUNDS",
            friendly_name: "PUBG: BATTLEGROUNDS",
        }),
        _ => Err("Unsupported NVIDIA game profile.".into()),
    }
}

fn target_for_executable(executable: &str) -> Result<NvidiaTarget, String> {
    match executable {
        "cs2.exe" => target_for_game("cs2"),
        "TslGame.exe" => target_for_game("pubg"),
        _ => Err("Unsupported NVIDIA executable in snapshot.".into()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaSetting {
    pub id: u32,
    pub key: String,
    pub value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaPayload {
    pub schema_version: u32,
    pub captured_at: String,
    pub profile_executable: String,
    pub settings: Vec<NvidiaSetting>,
    pub scaling_mode: String,
    pub scaling_value: u32,
    pub scaling_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaDiagnostics {
    pub gpu_detected: bool,
    pub gpu_name: Option<String>,
    pub driver_available: bool,
    pub driver_version: Option<u32>,
    pub driver_branch: Option<String>,
    pub nvapi_initialized: bool,
    pub cs2_profile_found: bool,
    pub cs2_profile_created: bool,
    pub profile_name: Option<String>,
    pub settings_read: usize,
    pub settings_applied: usize,
    pub settings_skipped: usize,
    pub settings_unsupported: usize,
    pub scaling_supported: bool,
    pub scaling_mode: Option<String>,
    pub scaling_result: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaCommandResponse {
    state: String,
    message: String,
    details: Vec<String>,
    retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<NvidiaPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<NvidiaDiagnostics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_token: Option<String>,
}

impl NvidiaCommandResponse {
    fn unsupported(message: &str) -> Self {
        Self {
            state: "unsupported".into(),
            message: message.into(),
            details: vec![],
            retryable: false,
            payload: None,
            diagnostics: None,
            backup_token: None,
        }
    }
}

pub fn scaling_name(value: u32) -> &'static str {
    match value {
        1 | 2 => "stretched",
        3 | 7 => "centered",
        5 | 6 => "aspect_ratio",
        8 => "integer_aspect",
        0 => "driver_default",
        _ => "unknown",
    }
}

pub fn availability_state(initialized: bool, gpu_found: bool) -> &'static str {
    if !initialized || !gpu_found {
        "unsupported"
    } else {
        "available"
    }
}

pub fn apply_state(
    settings_skipped: u32,
    settings_unsupported: u32,
    scaling_ok: bool,
) -> &'static str {
    if settings_skipped > 0 || settings_unsupported > 0 || !scaling_ok {
        "warning"
    } else {
        "success"
    }
}

pub fn validate_payload(payload: &NvidiaPayload) -> Result<(), String> {
    if payload.schema_version != 1 || target_for_executable(&payload.profile_executable).is_err() {
        return Err("Unsupported or corrupted NVIDIA snapshot.".into());
    }
    if payload.settings.len() > MAX_SETTINGS {
        return Err("NVIDIA snapshot contains too many settings.".into());
    }
    let mut seen = HashSet::new();
    for setting in &payload.settings {
        let expected = PORTABLE_SETTINGS
            .iter()
            .find(|(id, _)| *id == setting.id)
            .map(|(_, key)| *key)
            .ok_or_else(|| format!("Unknown NVIDIA setting id: 0x{:08X}", setting.id))?;
        if setting.key != expected {
            return Err(format!(
                "NVIDIA setting name mismatch for 0x{:08X}.",
                setting.id
            ));
        }
        if !seen.insert(setting.id) {
            return Err(format!("Duplicate NVIDIA setting id: 0x{:08X}", setting.id));
        }
    }
    if payload.scaling_supported && !matches!(payload.scaling_value, 0 | 1 | 2 | 3 | 5 | 6 | 7 | 8)
    {
        return Err("NVIDIA snapshot contains an unknown scaling mode.".into());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn capture(_request: NvidiaCaptureRequest) -> NvidiaCommandResponse {
    NvidiaCommandResponse::unsupported("NVIDIA settings are unsupported on this platform.")
}

#[cfg(not(target_os = "windows"))]
pub fn apply(_payload: NvidiaPayload) -> NvidiaCommandResponse {
    NvidiaCommandResponse::unsupported("NVIDIA settings are unsupported on this platform.")
}

#[cfg(not(target_os = "windows"))]
pub fn diagnostics() -> NvidiaCommandResponse {
    NvidiaCommandResponse::unsupported("NVIDIA settings are unsupported on this platform.")
}

#[cfg(not(target_os = "windows"))]
pub fn restore(_backup_token: Option<String>) -> NvidiaCommandResponse {
    NvidiaCommandResponse::unsupported("NVIDIA settings are unsupported on this platform.")
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use chrono::Utc;
    use std::{
        env,
        ffi::{c_char, CStr, OsStr},
        fs,
        path::{Path, PathBuf},
    };

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RawSetting {
        id: u32,
        value: u32,
        present: u32,
        key: [c_char; 64],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RawSnapshot {
        schema_version: u32,
        initialized: u32,
        gpu_found: u32,
        driver_version: u32,
        profile_found: u32,
        profile_created: u32,
        scaling_supported: u32,
        scaling: u32,
        setting_count: u32,
        gpu_name: [c_char; 128],
        driver_branch: [c_char; 64],
        profile_name: [c_char; 128],
        settings: [RawSetting; MAX_SETTINGS],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RawApplyReport {
        settings_applied: u32,
        settings_skipped: u32,
        settings_unsupported: u32,
        profile_found: u32,
        profile_created: u32,
        scaling_requested: u32,
        scaling_applied: u32,
        scaling_message: [c_char; 256],
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct BackupSetting {
        id: u32,
        key: String,
        value: u32,
        present: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct NvidiaBackup {
        schema_version: u32,
        captured_at: String,
        #[serde(default = "default_backup_executable")]
        profile_executable: String,
        profile_found: bool,
        scaling_supported: bool,
        scaling_value: u32,
        settings: Vec<BackupSetting>,
    }

    fn default_backup_executable() -> String {
        "cs2.exe".into()
    }

    extern "C" {
        fn gp_nvapi_capture(
            snapshot: *mut RawSnapshot,
            create_profile: u32,
            executable: *const u16,
            profile_name: *const u16,
            friendly_name: *const u16,
            error: *mut c_char,
            error_size: usize,
        ) -> i32;
        fn gp_nvapi_apply(
            snapshot: *const RawSnapshot,
            restore_mode: u32,
            executable: *const u16,
            profile_name: *const u16,
            friendly_name: *const u16,
            report: *mut RawApplyReport,
            error: *mut c_char,
            error_size: usize,
        ) -> i32;
    }

    pub fn capture(request: NvidiaCaptureRequest) -> NvidiaCommandResponse {
        let target = match target_for_game(&request.game) {
            Ok(value) => value,
            Err(message) => return error_response(message, false),
        };
        let raw = match capture_raw(true, target) {
            Ok(value) => value,
            Err((message, raw)) => return unavailable_or_error(message, raw),
        };
        let diagnostics = diagnostics_from_raw(&raw);
        let settings = present_settings(&raw);
        let scaling_supported = raw.scaling_supported != 0;
        let payload = NvidiaPayload {
            schema_version: 1,
            captured_at: Utc::now().to_rfc3339(),
            profile_executable: target.executable.into(),
            settings: settings.clone(),
            scaling_mode: scaling_name(raw.scaling).into(),
            scaling_value: raw.scaling,
            scaling_supported,
        };
        let has_warning = settings.is_empty() || !scaling_supported;
        let mut details = vec![format!("Portable DRS settings read: {}", settings.len())];
        if settings.is_empty() {
            details.push("The game profile currently has no explicit portable overrides.".into());
        }
        if !scaling_supported {
            details.push("NVIDIA Scaling — Unsupported for the primary display path.".into());
        } else {
            details.push(format!(
                "Captured scaling mode: {}",
                scaling_name(raw.scaling)
            ));
        }
        NvidiaCommandResponse {
            state: if has_warning { "warning" } else { "success" }.into(),
            message: format!(
                "Captured {} portable NVIDIA settings for {}.",
                settings.len(),
                target.friendly_name
            ),
            details,
            retryable: !scaling_supported,
            payload: Some(payload),
            diagnostics: Some(diagnostics),
            backup_token: None,
        }
    }

    pub fn apply(payload: NvidiaPayload) -> NvidiaCommandResponse {
        if let Err(message) = validate_payload(&payload) {
            return error_response(message, false);
        }
        let target = match target_for_executable(&payload.profile_executable) {
            Ok(value) => value,
            Err(message) => return error_response(message, false),
        };
        let before = match capture_raw(false, target) {
            Ok(value) => value,
            Err((message, raw)) => return unavailable_or_error(message, raw),
        };
        let backup = backup_from_raw(&before, target.executable);
        let backup_path = match write_backup(&backup) {
            Ok(value) => value,
            Err(message) => return error_response(message, true),
        };
        let desired = raw_from_payload(&payload, true);
        let report = match apply_raw(&desired, false, target) {
            Ok(value) => value,
            Err(message) => {
                let rollback = apply_raw(&before, true, target)
                    .map(|_| "Previous NVIDIA state was restored.".to_string())
                    .unwrap_or_else(|error| format!("NVIDIA rollback failed: {error}"));
                return NvidiaCommandResponse {
                    state: "error".into(),
                    message: "NVIDIA application profile could not be applied.".into(),
                    details: vec![message, rollback],
                    retryable: true,
                    payload: None,
                    diagnostics: None,
                    backup_token: Some(backup_path.to_string_lossy().into_owned()),
                };
            }
        };
        let scaling_ok = report.scaling_requested == 0 || report.scaling_applied != 0;
        let state = apply_state(
            report.settings_skipped,
            report.settings_unsupported,
            scaling_ok,
        );
        let mut details = vec![
            format!("DRS settings applied: {}", report.settings_applied),
            format!("DRS settings skipped: {}", report.settings_skipped),
            format!("DRS settings unsupported: {}", report.settings_unsupported),
            format!("Scaling: {}", char_array(&report.scaling_message)),
            format!("NVIDIA backup: {}", backup_path.display()),
        ];
        if payload.scaling_mode == "stretched" && scaling_ok {
            details.push(
                "NVAPI accepted Force GPU - Full Screen for the primary NVIDIA display path."
                    .into(),
            );
        }
        NvidiaCommandResponse {
            state: state.into(),
            message: if state == "warning" {
                "NVIDIA application profile was applied with unsupported or skipped settings."
                    .into()
            } else {
                format!(
                    "NVIDIA {} application profile and scaling were applied.",
                    target.friendly_name
                )
            },
            details,
            retryable: !scaling_ok,
            payload: None,
            diagnostics: Some(NvidiaDiagnostics {
                gpu_detected: true,
                gpu_name: nonempty(char_array(&before.gpu_name)),
                driver_available: true,
                driver_version: (before.driver_version != 0).then_some(before.driver_version),
                driver_branch: nonempty(char_array(&before.driver_branch)),
                nvapi_initialized: true,
                cs2_profile_found: report.profile_found != 0,
                cs2_profile_created: report.profile_created != 0,
                profile_name: Some(target.executable.into()),
                settings_read: payload.settings.len(),
                settings_applied: report.settings_applied as usize,
                settings_skipped: report.settings_skipped as usize,
                settings_unsupported: report.settings_unsupported as usize,
                scaling_supported: payload.scaling_supported,
                scaling_mode: Some(payload.scaling_mode),
                scaling_result: nonempty(char_array(&report.scaling_message)),
            }),
            backup_token: Some(backup_path.to_string_lossy().into_owned()),
        }
    }

    pub fn diagnostics() -> NvidiaCommandResponse {
        match capture_raw(false, target_for_game("cs2").expect("known NVIDIA target")) {
            Ok(raw) => NvidiaCommandResponse {
                state: "success".into(),
                message: "NVIDIA NVAPI diagnostics completed.".into(),
                details: vec![],
                retryable: false,
                payload: None,
                diagnostics: Some(diagnostics_from_raw(&raw)),
                backup_token: None,
            },
            Err((message, raw)) => unavailable_or_error(message, raw),
        }
    }

    pub fn restore(backup_token: Option<String>) -> NvidiaCommandResponse {
        let path = match backup_token {
            Some(value) => PathBuf::from(value),
            None => match latest_backup() {
                Ok(value) => value,
                Err(message) => return error_response(message, false),
            },
        };
        let backup = match read_backup(&path) {
            Ok(value) => value,
            Err(message) => return error_response(message, false),
        };
        let target = match target_for_executable(&backup.profile_executable) {
            Ok(value) => value,
            Err(message) => return error_response(message, false),
        };
        let raw = raw_from_backup(&backup);
        match apply_raw(&raw, true, target) {
            Ok(report) => {
                let scaling_ok = report.scaling_requested == 0 || report.scaling_applied != 0;
                NvidiaCommandResponse {
                    state: if scaling_ok && report.settings_skipped == 0 {
                        "success"
                    } else {
                        "warning"
                    }
                    .into(),
                    message: "Restored the latest NVIDIA state saved by Game Passport.".into(),
                    details: vec![
                        format!("Backup used: {}", path.display()),
                        format!("Settings restored/deleted: {}", report.settings_applied),
                        format!("Settings skipped: {}", report.settings_skipped),
                        format!("Scaling: {}", char_array(&report.scaling_message)),
                    ],
                    retryable: !scaling_ok,
                    payload: None,
                    diagnostics: None,
                    backup_token: None,
                }
            }
            Err(message) => error_response(format!("NVIDIA restore failed: {message}"), true),
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }

    fn capture_raw(
        create_profile: bool,
        target: NvidiaTarget,
    ) -> Result<RawSnapshot, (String, RawSnapshot)> {
        let mut snapshot: RawSnapshot = unsafe { std::mem::zeroed() };
        let mut error = [0 as c_char; 512];
        let executable = wide(target.executable);
        let profile_name = wide(target.profile_name);
        let friendly_name = wide(target.friendly_name);
        let status = unsafe {
            gp_nvapi_capture(
                &mut snapshot,
                u32::from(create_profile),
                executable.as_ptr(),
                profile_name.as_ptr(),
                friendly_name.as_ptr(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        if status == 0 {
            Ok(snapshot)
        } else {
            Err((char_array(&error), snapshot))
        }
    }

    fn apply_raw(
        snapshot: &RawSnapshot,
        restore: bool,
        target: NvidiaTarget,
    ) -> Result<RawApplyReport, String> {
        let mut report: RawApplyReport = unsafe { std::mem::zeroed() };
        let mut error = [0 as c_char; 512];
        let executable = wide(target.executable);
        let profile_name = wide(target.profile_name);
        let friendly_name = wide(target.friendly_name);
        let status = unsafe {
            gp_nvapi_apply(
                snapshot,
                u32::from(restore),
                executable.as_ptr(),
                profile_name.as_ptr(),
                friendly_name.as_ptr(),
                &mut report,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        if status == 0 {
            Ok(report)
        } else {
            Err(char_array(&error))
        }
    }

    fn present_settings(raw: &RawSnapshot) -> Vec<NvidiaSetting> {
        raw.settings
            .iter()
            .take(raw.setting_count.min(MAX_SETTINGS as u32) as usize)
            .filter(|setting| setting.present != 0)
            .map(|setting| NvidiaSetting {
                id: setting.id,
                key: char_array(&setting.key),
                value: setting.value,
            })
            .collect()
    }

    fn diagnostics_from_raw(raw: &RawSnapshot) -> NvidiaDiagnostics {
        NvidiaDiagnostics {
            gpu_detected: raw.gpu_found != 0,
            gpu_name: nonempty(char_array(&raw.gpu_name)),
            driver_available: raw.initialized != 0,
            driver_version: (raw.driver_version != 0).then_some(raw.driver_version),
            driver_branch: nonempty(char_array(&raw.driver_branch)),
            nvapi_initialized: raw.initialized != 0,
            cs2_profile_found: raw.profile_found != 0,
            cs2_profile_created: raw.profile_created != 0,
            profile_name: nonempty(char_array(&raw.profile_name)),
            settings_read: present_settings(raw).len(),
            settings_applied: 0,
            settings_skipped: 0,
            settings_unsupported: 0,
            scaling_supported: raw.scaling_supported != 0,
            scaling_mode: (raw.scaling_supported != 0).then(|| scaling_name(raw.scaling).into()),
            scaling_result: None,
        }
    }

    fn raw_from_payload(payload: &NvidiaPayload, profile_found: bool) -> RawSnapshot {
        let mut raw: RawSnapshot = unsafe { std::mem::zeroed() };
        raw.schema_version = 1;
        raw.profile_found = u32::from(profile_found);
        raw.scaling_supported = u32::from(payload.scaling_supported);
        raw.scaling = payload.scaling_value;
        raw.setting_count = payload.settings.len() as u32;
        for (index, setting) in payload.settings.iter().enumerate() {
            raw.settings[index].id = setting.id;
            raw.settings[index].value = setting.value;
            raw.settings[index].present = 1;
            write_char_array(&mut raw.settings[index].key, &setting.key);
        }
        raw
    }

    fn backup_from_raw(raw: &RawSnapshot, executable: &str) -> NvidiaBackup {
        NvidiaBackup {
            schema_version: 1,
            captured_at: Utc::now().to_rfc3339(),
            profile_executable: executable.into(),
            profile_found: raw.profile_found != 0,
            scaling_supported: raw.scaling_supported != 0,
            scaling_value: raw.scaling,
            settings: raw
                .settings
                .iter()
                .take(raw.setting_count.min(MAX_SETTINGS as u32) as usize)
                .map(|setting| BackupSetting {
                    id: setting.id,
                    key: char_array(&setting.key),
                    value: setting.value,
                    present: setting.present != 0,
                })
                .collect(),
        }
    }

    fn raw_from_backup(backup: &NvidiaBackup) -> RawSnapshot {
        let mut raw: RawSnapshot = unsafe { std::mem::zeroed() };
        raw.schema_version = 1;
        raw.profile_found = u32::from(backup.profile_found);
        raw.scaling_supported = u32::from(backup.scaling_supported);
        raw.scaling = backup.scaling_value;
        raw.setting_count = backup.settings.len().min(MAX_SETTINGS) as u32;
        for (index, setting) in backup.settings.iter().take(MAX_SETTINGS).enumerate() {
            raw.settings[index].id = setting.id;
            raw.settings[index].value = setting.value;
            raw.settings[index].present = u32::from(setting.present);
            write_char_array(&mut raw.settings[index].key, &setting.key);
        }
        raw
    }

    fn unavailable_or_error(message: String, raw: RawSnapshot) -> NvidiaCommandResponse {
        if availability_state(raw.initialized != 0, raw.gpu_found != 0) == "unsupported" {
            NvidiaCommandResponse {
                state: "unsupported".into(),
                message: "NVIDIA GPU or compatible NVIDIA Driver/NVAPI was not detected.".into(),
                details: nonempty(message).into_iter().collect(),
                retryable: false,
                payload: None,
                diagnostics: Some(diagnostics_from_raw(&raw)),
                backup_token: None,
            }
        } else {
            NvidiaCommandResponse {
                state: "error".into(),
                message: "NVIDIA NVAPI operation failed.".into(),
                details: nonempty(message).into_iter().collect(),
                retryable: true,
                payload: None,
                diagnostics: Some(diagnostics_from_raw(&raw)),
                backup_token: None,
            }
        }
    }

    fn backup_directory() -> Result<PathBuf, String> {
        let local = env::var_os("LOCALAPPDATA").ok_or_else(|| {
            "LOCALAPPDATA is unavailable; NVIDIA backup was not created.".to_string()
        })?;
        Ok(PathBuf::from(local)
            .join("Game Passport")
            .join("Backups")
            .join("NVIDIA"))
    }

    fn write_backup(backup: &NvidiaBackup) -> Result<PathBuf, String> {
        let directory = backup_directory()?;
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Could not create NVIDIA backup folder: {error}"))?;
        let path = directory.join(format!("{}.json", Utc::now().format("%Y%m%dT%H%M%S%.3fZ")));
        let json = serde_json::to_vec_pretty(backup)
            .map_err(|error| format!("Could not serialize NVIDIA backup: {error}"))?;
        fs::write(&path, json)
            .map_err(|error| format!("Could not write NVIDIA backup: {error}"))?;
        Ok(path)
    }

    fn latest_backup() -> Result<PathBuf, String> {
        let directory = backup_directory()?;
        let mut paths = fs::read_dir(&directory)
            .map_err(|_| "No Game Passport NVIDIA backup was found.".to_string())?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(OsStr::to_str) == Some("json"))
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .pop()
            .ok_or_else(|| "No Game Passport NVIDIA backup was found.".into())
    }

    fn read_backup(path: &Path) -> Result<NvidiaBackup, String> {
        let canonical_parent = backup_directory()?
            .canonicalize()
            .map_err(|error| format!("NVIDIA backup folder is unavailable: {error}"))?;
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("NVIDIA backup is unavailable: {error}"))?;
        if !canonical.starts_with(canonical_parent) {
            return Err(
                "NVIDIA backup token points outside the Game Passport backup folder.".into(),
            );
        }
        let bytes = fs::read(&canonical)
            .map_err(|error| format!("Could not read NVIDIA backup: {error}"))?;
        let backup: NvidiaBackup = serde_json::from_slice(&bytes)
            .map_err(|_| "NVIDIA backup is corrupted.".to_string())?;
        if backup.schema_version != 1 || backup.settings.len() > MAX_SETTINGS {
            return Err("NVIDIA backup is corrupted or unsupported.".into());
        }
        target_for_executable(&backup.profile_executable)
            .map_err(|_| "NVIDIA backup contains an unknown executable.".to_string())?;
        for setting in &backup.settings {
            if !PORTABLE_SETTINGS
                .iter()
                .any(|(id, key)| *id == setting.id && *key == setting.key)
            {
                return Err("NVIDIA backup contains an unknown setting.".into());
            }
        }
        Ok(backup)
    }

    fn char_array<const N: usize>(value: &[c_char; N]) -> String {
        unsafe { CStr::from_ptr(value.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    fn write_char_array<const N: usize>(destination: &mut [c_char; N], value: &str) {
        for (target, source) in destination
            .iter_mut()
            .take(N.saturating_sub(1))
            .zip(value.as_bytes())
        {
            *target = *source as c_char;
        }
    }

    fn nonempty(value: String) -> Option<String> {
        (!value.is_empty()).then_some(value)
    }

    fn error_response(message: String, retryable: bool) -> NvidiaCommandResponse {
        NvidiaCommandResponse {
            state: "error".into(),
            message,
            details: vec![],
            retryable,
            payload: None,
            diagnostics: None,
            backup_token: None,
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::{apply, capture, diagnostics, restore};

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> NvidiaPayload {
        NvidiaPayload {
            schema_version: 1,
            captured_at: "2026-01-01T00:00:00Z".into(),
            profile_executable: "cs2.exe".into(),
            settings: vec![NvidiaSetting {
                id: 0x1057EB71,
                key: "power_management_mode".into(),
                value: 1,
            }],
            scaling_mode: "stretched".into(),
            scaling_value: 2,
            scaling_supported: true,
        }
    }

    #[test]
    fn rejects_unknown_setting() {
        let mut value = payload();
        value.settings[0].id = 0xDEADBEEF;
        assert!(validate_payload(&value).is_err());
    }

    #[test]
    fn rejects_duplicate_setting() {
        let mut value = payload();
        value.settings.push(value.settings[0].clone());
        assert!(validate_payload(&value).is_err());
    }

    #[test]
    fn rejects_corrupted_snapshot() {
        let mut value = payload();
        value.schema_version = 99;
        assert!(validate_payload(&value).is_err());
    }

    #[test]
    fn accepts_pubg_executable_but_rejects_arbitrary_executables() {
        let mut value = payload();
        value.profile_executable = "TslGame.exe".into();
        assert!(validate_payload(&value).is_ok());
        value.profile_executable = "malware.exe".into();
        assert!(validate_payload(&value).is_err());
    }

    #[test]
    fn maps_full_screen_scaling_to_stretched() {
        assert_eq!(scaling_name(1), "stretched");
        assert_eq!(scaling_name(2), "stretched");
        assert_eq!(scaling_name(5), "aspect_ratio");
    }

    #[test]
    fn classifies_missing_gpu_or_nvapi_as_unsupported() {
        assert_eq!(availability_state(false, false), "unsupported");
        assert_eq!(availability_state(true, false), "unsupported");
        assert_eq!(availability_state(true, true), "available");
    }

    #[test]
    fn reports_incompatible_driver_settings_as_partial_success() {
        assert_eq!(apply_state(1, 0, true), "warning");
        assert_eq!(apply_state(0, 1, true), "warning");
        assert_eq!(apply_state(0, 0, false), "warning");
        assert_eq!(apply_state(0, 0, true), "success");
    }
}
