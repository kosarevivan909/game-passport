use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub bits_per_pixel: u32,
    pub interlaced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayPayload {
    pub schema_version: u32,
    pub captured_at: String,
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: String,
    pub display_mode: String,
    pub scaling_preference: String,
    pub refresh_rate_policy: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub struct DisplayCaptureRequest {
    pub width: u32,
    pub height: u32,
    pub display_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayDiagnostics {
    pub monitor_detected: bool,
    pub primary_monitor: Option<String>,
    pub primary_device: Option<String>,
    pub monitor_count: usize,
    pub current_mode: Option<DisplayMode>,
    pub supported_modes: Vec<DisplayMode>,
    pub selected_mode: Option<DisplayMode>,
    pub last_change_result: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayCommandResponse {
    state: String,
    message: String,
    details: Vec<String>,
    retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<DisplayPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<DisplayDiagnostics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_token: Option<String>,
}

impl DisplayCommandResponse {
    fn unsupported() -> Self {
        Self {
            state: "unsupported".into(),
            message: "Display settings are unsupported on this platform.".into(),
            details: vec![],
            retryable: false,
            payload: None,
            diagnostics: None,
            backup_token: None,
        }
    }
}

pub fn aspect_ratio(width: u32, height: u32) -> String {
    if width == 0 || height == 0 {
        return "unknown".into();
    }
    fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }
    let divisor = gcd(width, height);
    format!("{}:{}", width / divisor, height / divisor)
}

pub fn choose_best_mode(modes: &[DisplayMode], width: u32, height: u32) -> Option<DisplayMode> {
    let exact = modes
        .iter()
        .filter(|mode| mode.width == width && mode.height == height && mode.bits_per_pixel >= 32);
    let progressive_available = exact.clone().any(|mode| !mode.interlaced);
    exact
        .filter(|mode| !progressive_available || !mode.interlaced)
        .max_by_key(|mode| (mode.refresh_hz, mode.bits_per_pixel))
        .cloned()
}

pub fn choose_primary_index(primary_flags: &[bool]) -> Option<usize> {
    primary_flags
        .iter()
        .position(|primary| *primary)
        .or_else(|| (!primary_flags.is_empty()).then_some(0))
}

#[cfg(not(target_os = "windows"))]
pub fn capture(_request: DisplayCaptureRequest) -> DisplayCommandResponse {
    DisplayCommandResponse::unsupported()
}

#[cfg(not(target_os = "windows"))]
pub fn apply(_payload: DisplayPayload) -> DisplayCommandResponse {
    DisplayCommandResponse::unsupported()
}

#[cfg(not(target_os = "windows"))]
pub fn diagnostics() -> DisplayCommandResponse {
    DisplayCommandResponse::unsupported()
}

#[cfg(not(target_os = "windows"))]
pub fn restore(_backup_token: Option<String>) -> DisplayCommandResponse {
    DisplayCommandResponse::unsupported()
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use chrono::Utc;
    use std::{
        collections::HashSet,
        env,
        ffi::OsStr,
        fs,
        mem::{size_of, zeroed},
        os::windows::ffi::OsStrExt,
        path::{Path, PathBuf},
        ptr::{null, null_mut},
    };
    use windows_sys::Win32::Graphics::Gdi::{
        ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsExW, CDS_TEST,
        CDS_UPDATEREGISTRY, DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP,
        DISPLAY_DEVICE_PRIMARY_DEVICE, DISP_CHANGE_RESTART, DISP_CHANGE_SUCCESSFUL, DM_BITSPERPEL,
        DM_DISPLAYFLAGS, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
    };

    const DM_INTERLACED: u32 = 2;

    #[derive(Debug, Clone)]
    struct Monitor {
        device_name: String,
        friendly_name: String,
        primary: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DisplayBackup {
        schema_version: u32,
        captured_at: String,
        device_name: String,
        mode: DisplayMode,
    }

    pub fn capture(request: DisplayCaptureRequest) -> DisplayCommandResponse {
        match inspect(None) {
            Ok(diagnostics) => {
                let exact =
                    choose_best_mode(&diagnostics.supported_modes, request.width, request.height);
                let mut details = vec![format!(
                    "Primary gaming display: {}",
                    diagnostics
                        .primary_monitor
                        .as_deref()
                        .unwrap_or("Unknown monitor")
                )];
                let state = if exact.is_some() {
                    "success"
                } else {
                    "warning"
                };
                if exact.is_none() {
                    details.push(format!(
                        "{}x{} is not currently exposed by Windows for this display connection.",
                        request.width, request.height
                    ));
                }
                let payload = DisplayPayload {
                    schema_version: 1,
                    captured_at: Utc::now().to_rfc3339(),
                    width: request.width,
                    height: request.height,
                    aspect_ratio: aspect_ratio(request.width, request.height),
                    display_mode: request.display_mode,
                    scaling_preference: "driver_managed".into(),
                    refresh_rate_policy: "MAX_AVAILABLE".into(),
                };
                DisplayCommandResponse {
                    state: state.into(),
                    message: if exact.is_some() {
                        format!(
                            "Saved display policy {}x{} at maximum available refresh rate.",
                            request.width, request.height
                        )
                    } else {
                        "Saved the requested display policy, but it is unavailable on this monitor."
                            .into()
                    },
                    details,
                    retryable: exact.is_none(),
                    payload: Some(payload),
                    diagnostics: Some(diagnostics),
                    backup_token: None,
                }
            }
            Err(message) => error_response(message, true),
        }
    }

    pub fn apply(payload: DisplayPayload) -> DisplayCommandResponse {
        if let Err(message) = validate_payload(&payload) {
            return error_response(message, false);
        }
        let mut diagnostics = match inspect(Some((payload.width, payload.height))) {
            Ok(value) => value,
            Err(message) => return error_response(message, true),
        };
        let Some(selected) = diagnostics.selected_mode.clone() else {
            return DisplayCommandResponse {
                state: "warning".into(),
                message: format!(
                    "Resolution {}x{} is not supported on the primary display. Display mode was not changed.",
                    payload.width, payload.height
                ),
                details: vec![
                    "Other supported adapters may continue. No fallback resolution was selected."
                        .into(),
                ],
                retryable: false,
                payload: None,
                diagnostics: Some(diagnostics),
                backup_token: None,
            };
        };
        let monitor = match primary_monitor() {
            Ok(value) => value,
            Err(message) => return error_response(message, true),
        };
        let Some(current) = diagnostics.current_mode.clone() else {
            return error_response(
                "Windows did not report the current display mode.".into(),
                true,
            );
        };
        let backup = DisplayBackup {
            schema_version: 1,
            captured_at: Utc::now().to_rfc3339(),
            device_name: monitor.device_name.clone(),
            mode: current.clone(),
        };
        let backup_path = match write_backup(&backup) {
            Ok(path) => path,
            Err(message) => return error_response(message, true),
        };
        match set_mode(&monitor.device_name, &selected) {
            Ok(result) => {
                diagnostics.last_change_result = Some(result.clone());
                DisplayCommandResponse {
                    state: if result == "restart_required" {
                        "warning"
                    } else {
                        "success"
                    }
                    .into(),
                    message: format!(
                        "Applied {}x{} at {} Hz on the primary display.",
                        selected.width, selected.height, selected.refresh_hz
                    ),
                    details: vec![
                        "RefreshRatePolicy = MAX_AVAILABLE".into(),
                        format!("Display backup: {}", backup_path.display()),
                    ],
                    retryable: result == "restart_required",
                    payload: None,
                    diagnostics: Some(diagnostics),
                    backup_token: Some(backup_path.to_string_lossy().into_owned()),
                }
            }
            Err(message) => {
                let rollback = set_mode(&monitor.device_name, &current);
                diagnostics.last_change_result = Some("failed".into());
                DisplayCommandResponse {
                    state: "error".into(),
                    message: "Windows rejected the requested display mode.".into(),
                    details: vec![
                        message,
                        match rollback {
                            Ok(_) => "Previous display mode was restored.".into(),
                            Err(error) => format!("Display rollback also failed: {error}"),
                        },
                    ],
                    retryable: true,
                    payload: None,
                    diagnostics: Some(diagnostics),
                    backup_token: Some(backup_path.to_string_lossy().into_owned()),
                }
            }
        }
    }

    pub fn diagnostics() -> DisplayCommandResponse {
        match inspect(None) {
            Ok(value) => DisplayCommandResponse {
                state: "success".into(),
                message: "Display modes enumerated through Windows API.".into(),
                details: vec![],
                retryable: false,
                payload: None,
                diagnostics: Some(value),
                backup_token: None,
            },
            Err(message) => error_response(message, true),
        }
    }

    pub fn restore(backup_token: Option<String>) -> DisplayCommandResponse {
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
        match set_mode(&backup.device_name, &backup.mode) {
            Ok(_) => DisplayCommandResponse {
                state: "success".into(),
                message: format!(
                    "Restored display mode {}x{} at {} Hz.",
                    backup.mode.width, backup.mode.height, backup.mode.refresh_hz
                ),
                details: vec![format!("Backup used: {}", path.display())],
                retryable: false,
                payload: None,
                diagnostics: inspect(None).ok(),
                backup_token: None,
            },
            Err(message) => error_response(format!("Display restore failed: {message}"), true),
        }
    }

    fn inspect(desired: Option<(u32, u32)>) -> Result<DisplayDiagnostics, String> {
        let monitors = enumerate_monitors();
        let monitor = choose_primary_index(
            &monitors
                .iter()
                .map(|monitor| monitor.primary)
                .collect::<Vec<_>>(),
        )
        .and_then(|index| monitors.get(index).cloned())
        .ok_or_else(|| "Windows did not report an attached desktop display.".to_string())?;
        let current = current_mode(&monitor.device_name);
        let modes = enumerate_modes(&monitor.device_name);
        let selected = desired.and_then(|(width, height)| choose_best_mode(&modes, width, height));
        Ok(DisplayDiagnostics {
            monitor_detected: true,
            primary_monitor: Some(monitor.friendly_name),
            primary_device: Some(monitor.device_name),
            monitor_count: monitors.len(),
            current_mode: current,
            supported_modes: modes,
            selected_mode: selected,
            last_change_result: None,
        })
    }

    fn enumerate_monitors() -> Vec<Monitor> {
        let mut monitors = Vec::new();
        let mut index = 0;
        loop {
            let mut adapter: DISPLAY_DEVICEW = unsafe { zeroed() };
            adapter.cb = size_of::<DISPLAY_DEVICEW>() as u32;
            if unsafe { EnumDisplayDevicesW(null(), index, &mut adapter, 0) } == 0 {
                break;
            }
            index += 1;
            if adapter.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0 {
                continue;
            }
            let device_name = wide_array_to_string(&adapter.DeviceName);
            let device_wide = wide_null(&device_name);
            let mut display: DISPLAY_DEVICEW = unsafe { zeroed() };
            display.cb = size_of::<DISPLAY_DEVICEW>() as u32;
            let friendly_name =
                if unsafe { EnumDisplayDevicesW(device_wide.as_ptr(), 0, &mut display, 0) } != 0 {
                    let value = wide_array_to_string(&display.DeviceString);
                    if value.is_empty() {
                        wide_array_to_string(&adapter.DeviceString)
                    } else {
                        value
                    }
                } else {
                    wide_array_to_string(&adapter.DeviceString)
                };
            monitors.push(Monitor {
                device_name,
                friendly_name,
                primary: adapter.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE != 0,
            });
        }
        monitors
    }

    fn primary_monitor() -> Result<Monitor, String> {
        let monitors = enumerate_monitors();
        choose_primary_index(
            &monitors
                .iter()
                .map(|monitor| monitor.primary)
                .collect::<Vec<_>>(),
        )
        .and_then(|index| monitors.get(index).cloned())
        .ok_or_else(|| "Primary display was not detected.".into())
    }

    fn current_mode(device_name: &str) -> Option<DisplayMode> {
        let mut raw: DEVMODEW = unsafe { zeroed() };
        raw.dmSize = size_of::<DEVMODEW>() as u16;
        let device = wide_null(device_name);
        if unsafe { EnumDisplaySettingsExW(device.as_ptr(), ENUM_CURRENT_SETTINGS, &mut raw, 0) }
            == 0
        {
            None
        } else {
            Some(mode_from_devmode(&raw))
        }
    }

    fn enumerate_modes(device_name: &str) -> Vec<DisplayMode> {
        let device = wide_null(device_name);
        let mut index = 0;
        let mut modes = Vec::new();
        let mut seen = HashSet::new();
        loop {
            let mut raw: DEVMODEW = unsafe { zeroed() };
            raw.dmSize = size_of::<DEVMODEW>() as u16;
            if unsafe { EnumDisplaySettingsExW(device.as_ptr(), index, &mut raw, 0) } == 0 {
                break;
            }
            index += 1;
            let mode = mode_from_devmode(&raw);
            if mode.width == 0
                || mode.height == 0
                || mode.refresh_hz <= 1
                || mode.bits_per_pixel < 32
            {
                continue;
            }
            let key = (
                mode.width,
                mode.height,
                mode.refresh_hz,
                mode.bits_per_pixel,
                mode.interlaced,
            );
            if seen.insert(key) {
                modes.push(mode);
            }
        }
        modes.sort_by_key(|mode| {
            (
                mode.width,
                mode.height,
                mode.refresh_hz,
                mode.bits_per_pixel,
            )
        });
        modes
    }

    fn mode_from_devmode(raw: &DEVMODEW) -> DisplayMode {
        DisplayMode {
            width: raw.dmPelsWidth,
            height: raw.dmPelsHeight,
            refresh_hz: raw.dmDisplayFrequency,
            bits_per_pixel: raw.dmBitsPerPel,
            interlaced: unsafe { raw.Anonymous2.dmDisplayFlags } & DM_INTERLACED != 0,
        }
    }

    fn set_mode(device_name: &str, mode: &DisplayMode) -> Result<String, String> {
        let device = wide_null(device_name);
        let mut raw: DEVMODEW = unsafe { zeroed() };
        raw.dmSize = size_of::<DEVMODEW>() as u16;
        raw.dmFields =
            DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_BITSPERPEL | DM_DISPLAYFLAGS;
        raw.dmPelsWidth = mode.width;
        raw.dmPelsHeight = mode.height;
        raw.dmDisplayFrequency = mode.refresh_hz;
        raw.dmBitsPerPel = mode.bits_per_pixel;
        raw.Anonymous2.dmDisplayFlags = if mode.interlaced { DM_INTERLACED } else { 0 };
        let test = unsafe {
            ChangeDisplaySettingsExW(device.as_ptr(), &raw, null_mut(), CDS_TEST, null_mut())
        };
        if test != DISP_CHANGE_SUCCESSFUL {
            return Err(format!("CDS_TEST returned {test}."));
        }
        let result = unsafe {
            ChangeDisplaySettingsExW(
                device.as_ptr(),
                &raw,
                null_mut(),
                CDS_UPDATEREGISTRY,
                null_mut(),
            )
        };
        match result {
            DISP_CHANGE_SUCCESSFUL => Ok("applied".into()),
            DISP_CHANGE_RESTART => Ok("restart_required".into()),
            value => Err(format!("ChangeDisplaySettingsExW returned {value}.")),
        }
    }

    fn validate_payload(payload: &DisplayPayload) -> Result<(), String> {
        if payload.schema_version != 1 {
            return Err("Unsupported or corrupted display snapshot schema.".into());
        }
        if !(320..=16384).contains(&payload.width) || !(200..=16384).contains(&payload.height) {
            return Err("Display snapshot contains an invalid resolution.".into());
        }
        if payload.refresh_rate_policy != "MAX_AVAILABLE" {
            return Err("Only RefreshRatePolicy = MAX_AVAILABLE is accepted.".into());
        }
        if !matches!(
            payload.display_mode.as_str(),
            "fullscreen" | "borderless" | "windowed" | "unknown"
        ) {
            return Err("Display snapshot contains an invalid window mode.".into());
        }
        Ok(())
    }

    fn backup_directory() -> Result<PathBuf, String> {
        let local = env::var_os("LOCALAPPDATA").ok_or_else(|| {
            "LOCALAPPDATA is unavailable; display backup was not created.".to_string()
        })?;
        Ok(PathBuf::from(local)
            .join("Game Passport")
            .join("Backups")
            .join("Display"))
    }

    fn write_backup(backup: &DisplayBackup) -> Result<PathBuf, String> {
        let directory = backup_directory()?;
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Could not create display backup folder: {error}"))?;
        let path = directory.join(format!("{}.json", Utc::now().format("%Y%m%dT%H%M%S%.3fZ")));
        let json = serde_json::to_vec_pretty(backup)
            .map_err(|error| format!("Could not serialize display backup: {error}"))?;
        fs::write(&path, json)
            .map_err(|error| format!("Could not write display backup: {error}"))?;
        Ok(path)
    }

    fn latest_backup() -> Result<PathBuf, String> {
        let directory = backup_directory()?;
        let mut paths = fs::read_dir(&directory)
            .map_err(|_| "No Game Passport display backup was found.".to_string())?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(OsStr::to_str) == Some("json"))
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .pop()
            .ok_or_else(|| "No Game Passport display backup was found.".into())
    }

    fn read_backup(path: &Path) -> Result<DisplayBackup, String> {
        let canonical_parent = backup_directory()?
            .canonicalize()
            .map_err(|error| format!("Display backup folder is unavailable: {error}"))?;
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("Display backup is unavailable: {error}"))?;
        if !canonical.starts_with(canonical_parent) {
            return Err(
                "Display backup token points outside the Game Passport backup folder.".into(),
            );
        }
        let bytes = fs::read(&canonical)
            .map_err(|error| format!("Could not read display backup: {error}"))?;
        let backup: DisplayBackup = serde_json::from_slice(&bytes)
            .map_err(|_| "Display backup is corrupted.".to_string())?;
        if backup.schema_version != 1 {
            return Err("Display backup schema is unsupported.".into());
        }
        Ok(backup)
    }

    fn wide_null(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    fn wide_array_to_string(value: &[u16]) -> String {
        let end = value
            .iter()
            .position(|char| *char == 0)
            .unwrap_or(value.len());
        String::from_utf16_lossy(&value[..end])
    }

    fn error_response(message: String, retryable: bool) -> DisplayCommandResponse {
        DisplayCommandResponse {
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

    fn mode(width: u32, height: u32, hz: u32, interlaced: bool) -> DisplayMode {
        DisplayMode {
            width,
            height,
            refresh_hz: hz,
            bits_per_pixel: 32,
            interlaced,
        }
    }

    #[test]
    fn chooses_highest_progressive_refresh_for_exact_resolution() {
        let modes = vec![
            mode(1280, 960, 144, false),
            mode(1280, 960, 400, true),
            mode(1280, 960, 360, false),
            mode(1920, 1080, 500, false),
        ];
        assert_eq!(
            choose_best_mode(&modes, 1280, 960),
            Some(mode(1280, 960, 360, false))
        );
    }

    #[test]
    fn returns_none_instead_of_silent_fallback() {
        assert_eq!(
            choose_best_mode(&[mode(1920, 1080, 240, false)], 1280, 960),
            None
        );
    }

    #[test]
    fn handles_duplicate_refresh_rates_deterministically() {
        let mut high_bpp = mode(1280, 960, 240, false);
        high_bpp.bits_per_pixel = 64;
        assert_eq!(
            choose_best_mode(&[mode(1280, 960, 240, false), high_bpp.clone()], 1280, 960),
            Some(high_bpp)
        );
    }

    #[test]
    fn normalizes_aspect_ratio() {
        assert_eq!(aspect_ratio(1280, 960), "4:3");
        assert_eq!(aspect_ratio(1920, 1080), "16:9");
    }

    #[test]
    fn selects_only_the_primary_display_in_multi_monitor_setup() {
        assert_eq!(choose_primary_index(&[false, true, false]), Some(1));
        assert_eq!(choose_primary_index(&[false, false]), Some(0));
        assert_eq!(choose_primary_index(&[]), None);
    }
}
