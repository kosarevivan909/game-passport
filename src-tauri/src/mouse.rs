use serde::{Deserialize, Serialize};

const MIN_SAFE_DPI: u32 = 50;
const MAX_SAFE_DPI: u32 = 100_000;
const MIN_SAFE_POLLING: u32 = 125;
const MAX_SAFE_POLLING: u32 = 8_000;
const RAZER_IMPLEMENTED_PIDS: &[u16] = &[0x00A5, 0x00A6, 0x00B2, 0x00B6, 0x00B7, 0x00C0, 0x00C1];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAdapterKind {
    LogitechProbe,
    Razer,
    LamzuDetectionOnly,
    Unsupported,
}

pub fn classify_vid_pid(vendor_id: u16, product_id: u16) -> MouseAdapterKind {
    match vendor_id {
        0x046D => MouseAdapterKind::LogitechProbe,
        0x1532 if RAZER_IMPLEMENTED_PIDS.contains(&product_id) => MouseAdapterKind::Razer,
        0x373E => MouseAdapterKind::LamzuDetectionOnly,
        _ => MouseAdapterKind::Unsupported,
    }
}

pub fn apply_outcome_state(
    dpi_verified: bool,
    polling_requested: bool,
    polling_verified: bool,
    normalized: bool,
) -> &'static str {
    if !dpi_verified {
        "warning"
    } else if normalized || (polling_requested && !polling_verified) {
        "warning"
    } else {
        "success"
    }
}

fn validate_backup_values(schema_version: u32, dpi: u32, polling: Option<u32>) -> bool {
    schema_version == 1
        && (MIN_SAFE_DPI..=MAX_SAFE_DPI).contains(&dpi)
        && polling
            .map(|rate| (MIN_SAFE_POLLING..=MAX_SAFE_POLLING).contains(&rate))
            .unwrap_or(true)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MousePayload {
    pub schema_version: u32,
    pub captured_at: String,
    pub dpi: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polling_rate_hz: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DpiCapabilities {
    pub minimum: u32,
    pub maximum: u32,
    pub step: u32,
    #[serde(default)]
    pub values: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseCapabilities {
    pub can_read_dpi: bool,
    pub can_apply_dpi: bool,
    pub can_read_polling_rate: bool,
    pub can_apply_polling_rate: bool,
    pub can_verify: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpi: Option<DpiCapabilities>,
    #[serde(default)]
    pub polling_rates_hz: Vec<u32>,
    pub reason: Option<String>,
}

impl MouseCapabilities {
    fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            can_read_dpi: false,
            can_apply_dpi: false,
            can_read_polling_rate: false,
            can_apply_polling_rate: false,
            can_verify: false,
            dpi: None,
            polling_rates_hz: vec![],
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseDeviceDiagnostics {
    pub instance_id: String,
    pub vendor_id: String,
    pub product_id: String,
    pub manufacturer: String,
    pub model: String,
    pub connection: String,
    pub hid_usage: String,
    pub selected_adapter: String,
    pub selected: bool,
    pub capabilities: MouseCapabilities,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseDiagnostics {
    pub devices: Vec<MouseDeviceDiagnostics>,
    #[serde(default)]
    pub probe_errors: Vec<String>,
    pub selected_instance_id: Option<String>,
    pub selection_ambiguous: bool,
    pub current_dpi: Option<u32>,
    pub requested_dpi: Option<u32>,
    pub applied_dpi: Option<u32>,
    pub current_polling_rate_hz: Option<u32>,
    pub requested_polling_rate_hz: Option<u32>,
    pub applied_polling_rate_hz: Option<u32>,
    pub verification_result: Option<String>,
    pub backup_result: Option<String>,
    pub restore_result: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseCommandResponse {
    state: String,
    message: String,
    details: Vec<String>,
    retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<MousePayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<MouseDiagnostics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_token: Option<String>,
}

impl MouseCommandResponse {
    fn unsupported() -> Self {
        Self {
            state: "unsupported".into(),
            message: "Mouse hardware functions are unsupported on this platform.".into(),
            details: vec![],
            retryable: false,
            payload: None,
            diagnostics: None,
            backup_token: None,
        }
    }
}

pub fn validate_payload(payload: &MousePayload) -> Result<(), String> {
    if payload.schema_version != 1 {
        return Err("Unsupported or corrupted Mouse snapshot.".into());
    }
    if !(MIN_SAFE_DPI..=MAX_SAFE_DPI).contains(&payload.dpi) {
        return Err("Mouse snapshot contains an unsafe DPI value.".into());
    }
    if let Some(rate) = payload.polling_rate_hz {
        if !(MIN_SAFE_POLLING..=MAX_SAFE_POLLING).contains(&rate) {
            return Err("Mouse snapshot contains an unsafe polling-rate value.".into());
        }
    }
    Ok(())
}

pub fn normalize_desired_dpi(desired: u32, capabilities: &DpiCapabilities) -> Option<u32> {
    if !capabilities.values.is_empty() {
        return capabilities
            .values
            .iter()
            .copied()
            .min_by_key(|candidate| (candidate.abs_diff(desired), *candidate));
    }
    if capabilities.minimum > capabilities.maximum || capabilities.step == 0 {
        return None;
    }
    let clamped = desired.clamp(capabilities.minimum, capabilities.maximum);
    let offset = clamped - capabilities.minimum;
    let lower = capabilities.minimum + (offset / capabilities.step) * capabilities.step;
    let upper = lower
        .saturating_add(capabilities.step)
        .min(capabilities.maximum);
    Some(if lower.abs_diff(clamped) <= upper.abs_diff(clamped) {
        lower
    } else {
        upper
    })
}

pub fn normalize_polling_rate(desired: u32, supported: &[u32]) -> Option<u32> {
    let mut rates: Vec<u32> = supported.iter().copied().filter(|rate| *rate > 0).collect();
    rates.sort_unstable();
    rates.dedup();
    rates
        .iter()
        .copied()
        .filter(|rate| *rate <= desired)
        .max()
        .or_else(|| rates.first().copied())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionFixture {
    pub instance_id: String,
    pub controllable: bool,
}

pub fn select_unique_controllable(devices: &[SelectionFixture]) -> Result<Option<usize>, String> {
    let mut candidates: Vec<(usize, &str)> = devices
        .iter()
        .enumerate()
        .filter(|(_, device)| device.controllable)
        .map(|(index, device)| (index, device.instance_id.as_str()))
        .collect();
    candidates.sort_by_key(|(_, instance)| *instance);
    candidates.dedup_by_key(|(_, instance)| *instance);
    match candidates.as_slice() {
        [] => Ok(None),
        [(index, _)] => Ok(Some(*index)),
        _ => Err("Multiple controllable gaming mice were detected.".into()),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn capture() -> MouseCommandResponse {
    MouseCommandResponse::unsupported()
}

#[cfg(not(target_os = "windows"))]
pub fn apply(_payload: MousePayload) -> MouseCommandResponse {
    MouseCommandResponse::unsupported()
}

#[cfg(not(target_os = "windows"))]
pub fn diagnostics() -> MouseCommandResponse {
    MouseCommandResponse::unsupported()
}

#[cfg(not(target_os = "windows"))]
pub fn restore(_backup_token: Option<String>) -> MouseCommandResponse {
    MouseCommandResponse::unsupported()
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use chrono::Utc;
    use hidapi::{HidApi, HidDevice};
    use sha2::{Digest, Sha256};
    use std::{
        collections::{BTreeMap, HashSet},
        env,
        ffi::CString,
        fs,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
        thread,
        time::Duration,
    };

    const LOGITECH_VID: u16 = 0x046D;
    const RAZER_VID: u16 = 0x1532;
    const LAMZU_VID: u16 = 0x373E;
    const HIDPP_DPI: u16 = 0x2201;
    const HIDPP_REPORT_RATE: u16 = 0x8060;

    static LAST_DIAGNOSTICS: OnceLock<Mutex<Option<MouseDiagnostics>>> = OnceLock::new();

    #[derive(Clone)]
    struct Endpoint {
        path: CString,
        vendor_id: u16,
        product_id: u16,
        manufacturer: String,
        product: String,
        usage_page: u16,
        usage: u16,
        interface_number: i32,
        instance_id: String,
        physical_key: String,
    }

    #[derive(Clone, Copy)]
    struct RazerModel {
        product_id: u16,
        name: &'static str,
        connection: &'static str,
        maximum_dpi: u32,
        polling2: bool,
        polling_rates: &'static [u32],
    }

    const RAZER_MODELS: &[RazerModel] = &[
        RazerModel {
            product_id: 0x00A5,
            name: "Viper V2 Pro (wired)",
            connection: "USB cable",
            maximum_dpi: 30_000,
            polling2: false,
            polling_rates: &[125, 500, 1000],
        },
        RazerModel {
            product_id: 0x00A6,
            name: "Viper V2 Pro",
            connection: "wireless receiver",
            maximum_dpi: 30_000,
            polling2: false,
            polling_rates: &[125, 500, 1000],
        },
        RazerModel {
            product_id: 0x00B2,
            name: "DeathAdder V3 (wired)",
            connection: "USB cable",
            maximum_dpi: 30_000,
            polling2: true,
            polling_rates: &[125, 250, 500, 1000, 2000, 4000, 8000],
        },
        RazerModel {
            product_id: 0x00B6,
            name: "DeathAdder V3 Pro (wired)",
            connection: "USB cable",
            maximum_dpi: 30_000,
            polling2: true,
            polling_rates: &[125, 250, 500, 1000, 2000, 4000],
        },
        RazerModel {
            product_id: 0x00B7,
            name: "DeathAdder V3 Pro",
            connection: "wireless receiver",
            maximum_dpi: 30_000,
            polling2: true,
            polling_rates: &[125, 250, 500, 1000, 2000, 4000],
        },
        RazerModel {
            product_id: 0x00C0,
            name: "Viper V3 Pro (wired)",
            connection: "USB cable",
            maximum_dpi: 35_000,
            polling2: true,
            polling_rates: &[125, 250, 500, 1000, 2000, 4000, 8000],
        },
        RazerModel {
            product_id: 0x00C1,
            name: "Viper V3 Pro",
            connection: "wireless receiver",
            maximum_dpi: 35_000,
            polling2: true,
            polling_rates: &[125, 250, 500, 1000, 2000, 4000, 8000],
        },
    ];

    #[derive(Clone)]
    enum Control {
        Logitech {
            path: CString,
            device_index: u8,
            dpi_feature: u8,
            rate_feature: Option<u8>,
        },
        Razer {
            path: CString,
            model: RazerModel,
        },
    }

    /// One hardware-only contract shared by brand transports. Detection and cloud
    /// persistence remain outside it so device identity cannot enter a profile.
    trait MouseHardwareAdapter {
        fn read_current_settings(&self) -> Result<(u32, Option<u32>), String>;
        fn apply_dpi(&self, dpi: u32) -> Result<(), String>;
        fn apply_polling_rate(&self, rate: u32) -> Result<(), String>;
        fn verify_settings(&self, dpi: u32, polling: Option<u32>) -> Result<(), String> {
            let (actual_dpi, actual_polling) = self.read_current_settings()?;
            if actual_dpi != dpi {
                return Err(format!(
                    "DPI readback mismatch: expected {dpi}, got {actual_dpi}."
                ));
            }
            if let Some(expected) = polling {
                if actual_polling != Some(expected) {
                    return Err(format!(
                        "Polling readback mismatch: expected {expected}, got {actual_polling:?}."
                    ));
                }
            }
            Ok(())
        }
    }

    #[derive(Clone)]
    struct Target {
        diagnostics: MouseDeviceDiagnostics,
        control: Control,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MouseBackup {
        schema_version: u32,
        captured_at: String,
        instance_id: String,
        dpi: u32,
        polling_rate_hz: Option<u32>,
    }

    pub fn diagnostics() -> MouseCommandResponse {
        match inspect() {
            Ok((mut diagnostic, selection)) => {
                if let Ok(Some(target)) = selection {
                    if let Ok((dpi, polling)) = read_settings(&target.control) {
                        diagnostic.current_dpi = Some(dpi);
                        diagnostic.current_polling_rate_hz = polling;
                    }
                }
                merge_last(&mut diagnostic);
                remember(&diagnostic);
                let state = if diagnostic.selection_ambiguous
                    || diagnostic.selected_instance_id.is_none()
                {
                    "warning"
                } else {
                    "success"
                };
                response(
                    state,
                    if diagnostic.selection_ambiguous {
                        "Обнаружено несколько управляемых мышей — безопасный выбор не выполнен."
                    } else if diagnostic.selected_instance_id.is_some() {
                        "Мышь обнаружена, текущие параметры считаны."
                    } else {
                        "Совместимая мышь не найдена. Откройте сведения ниже."
                    },
                    vec![],
                    Some(diagnostic),
                    None,
                    None,
                )
            }
            Err(message) => response("error", &message, vec![], None, None, None),
        }
    }

    pub fn capture() -> MouseCommandResponse {
        let (mut diagnostic, selection) = match inspect() {
            Ok(result) => result,
            Err(message) => return response("error", &message, vec![], None, None, None),
        };
        let target = match selection {
            Ok(Some(target)) => target,
            Ok(None) => {
                return manual_response(
                    &mut diagnostic,
                    None,
                    "Мышь обнаружена, но её DPI не удалось безопасно считать автоматически.",
                )
            }
            Err(message) => return manual_response(&mut diagnostic, None, &message),
        };
        match read_settings(&target.control) {
            Ok((dpi, polling)) => {
                diagnostic.current_dpi = Some(dpi);
                diagnostic.current_polling_rate_hz = polling;
                diagnostic.verification_result =
                    Some("Current hardware settings were read from the selected mouse.".into());
                remember(&diagnostic);
                let payload = MousePayload {
                    schema_version: 1,
                    captured_at: Utc::now().to_rfc3339(),
                    dpi,
                    polling_rate_hz: polling,
                };
                let state = if polling.is_some() {
                    "success"
                } else {
                    "warning"
                };
                response(
                    state,
                    if polling.is_some() {
                        "Текущие DPI и частота опроса мыши сохранены."
                    } else {
                        "DPI мыши сохранён; частоту опроса считать не удалось."
                    },
                    vec![
                        format!("DPI: {dpi}"),
                        polling
                            .map(|rate| format!("Polling rate: {rate} Hz"))
                            .unwrap_or_else(|| "Частота опроса: недоступна".into()),
                    ],
                    Some(diagnostic),
                    Some(payload),
                    None,
                )
            }
            Err(message) => manual_response(
                &mut diagnostic,
                None,
                &format!("Mouse settings could not be read: {message}"),
            ),
        }
    }

    pub fn apply(payload: MousePayload) -> MouseCommandResponse {
        if let Err(message) = validate_payload(&payload) {
            return response("error", &message, vec![], None, None, None);
        }
        let (mut diagnostic, selection) = match inspect() {
            Ok(result) => result,
            Err(message) => return response("error", &message, vec![], None, None, None),
        };
        diagnostic.requested_dpi = Some(payload.dpi);
        diagnostic.requested_polling_rate_hz = payload.polling_rate_hz;
        let target = match selection {
            Ok(Some(target)) => target,
            Ok(None) => {
                return manual_response(
                    &mut diagnostic,
                    Some(&payload),
                    "No supported gaming mouse was found.",
                )
            }
            Err(message) => return manual_response(&mut diagnostic, Some(&payload), &message),
        };
        let capabilities = &target.diagnostics.capabilities;
        let desired_dpi = match capabilities
            .dpi
            .as_ref()
            .and_then(|caps| normalize_desired_dpi(payload.dpi, caps))
        {
            Some(value) => value,
            None => {
                return manual_response(
                    &mut diagnostic,
                    Some(&payload),
                    "DPI is unsupported for the selected mouse.",
                )
            }
        };
        let desired_rate = payload
            .polling_rate_hz
            .and_then(|rate| normalize_polling_rate(rate, &capabilities.polling_rates_hz));
        let before = read_settings(&target.control).ok();
        let backup_token = before.as_ref().and_then(|(dpi, polling)| {
            match write_backup(&target.diagnostics.instance_id, *dpi, *polling) {
                Ok(token) => {
                    diagnostic.backup_result = Some("Local pre-Game Passport backup saved.".into());
                    Some(token)
                }
                Err(message) => {
                    diagnostic.backup_result = Some(format!("Backup failed: {message}"));
                    None
                }
            }
        });
        let mut warnings = Vec::new();
        if desired_dpi != payload.dpi {
            warnings.push(format!(
                "Requested: {} DPI; applied nearest supported value: {} DPI.",
                payload.dpi, desired_dpi
            ));
        }
        if let Some(requested) = payload.polling_rate_hz {
            match desired_rate {
                Some(applied) if applied != requested => warnings.push(format!(
                    "Requested: {requested} Hz; maximum/nearest supported: {applied} Hz."
                )),
                None => warnings.push(format!(
                    "Polling rate is unsupported; set it manually to {requested} Hz."
                )),
                _ => {}
            }
        }
        if let Err(message) = set_dpi(&target.control, desired_dpi) {
            diagnostic.verification_result = Some(format!("DPI write failed: {message}"));
            return manual_response(
                &mut diagnostic,
                Some(&payload),
                "The mouse rejected the DPI change.",
            );
        }
        if let Some(rate) = desired_rate {
            if let Err(message) = set_polling(&target.control, rate) {
                warnings.push(format!("Polling-rate write failed: {message}"));
            }
        }
        if let Err(message) = target.control.verify_settings(desired_dpi, None) {
            diagnostic.verification_result = Some(message);
            return manual_response(
                &mut diagnostic,
                Some(&payload),
                "The DPI change could not be verified by hardware readback.",
            );
        }
        let (verified_dpi, verified_rate) = match read_settings(&target.control) {
            Ok(values) => values,
            Err(message) => {
                diagnostic.verification_result = Some(format!("Readback failed: {message}"));
                return manual_response(
                    &mut diagnostic,
                    Some(&payload),
                    "The mouse change could not be verified.",
                );
            }
        };
        diagnostic.current_dpi = Some(verified_dpi);
        diagnostic.current_polling_rate_hz = verified_rate;
        if verified_dpi != desired_dpi {
            diagnostic.verification_result = Some(format!(
                "DPI readback mismatch: expected {desired_dpi}, got {verified_dpi}."
            ));
            return manual_response(&mut diagnostic, Some(&payload), "DPI verification failed.");
        }
        diagnostic.applied_dpi = Some(verified_dpi);
        if let Some(expected) = desired_rate {
            if verified_rate == Some(expected) {
                diagnostic.applied_polling_rate_hz = verified_rate;
            } else {
                warnings.push(format!(
                    "Polling-rate readback did not confirm {expected} Hz."
                ));
            }
        }
        diagnostic.verification_result =
            Some("DPI hardware readback confirmed; polling is reported separately.".into());
        remember(&diagnostic);
        let state = apply_outcome_state(
            true,
            payload.polling_rate_hz.is_some(),
            payload.polling_rate_hz.is_none() || diagnostic.applied_polling_rate_hz.is_some(),
            !warnings.is_empty(),
        );
        let mut details = vec![
            format!("Requested DPI: {}", payload.dpi),
            format!("Applied and verified DPI: {verified_dpi}"),
        ];
        details.extend(warnings.clone());
        response(
            state,
            if warnings.is_empty() {
                "Mouse settings were applied and verified."
            } else {
                "Mouse DPI was applied, with limitations."
            },
            details,
            Some(diagnostic),
            None,
            backup_token,
        )
    }

    pub fn restore(backup_token: Option<String>) -> MouseCommandResponse {
        let backup = match read_backup(backup_token.as_deref()) {
            Ok(value) => value,
            Err(message) => return response("warning", &message, vec![], None, None, None),
        };
        let (mut diagnostic, selection) = match inspect() {
            Ok(result) => result,
            Err(message) => return response("error", &message, vec![], None, None, None),
        };
        let target = match selection {
            Ok(Some(target)) if target.diagnostics.instance_id == backup.instance_id => target,
            Ok(Some(_)) => return manual_response(&mut diagnostic, None, "The backed-up physical mouse is not currently selected; restore was not attempted."),
            Ok(None) => return manual_response(&mut diagnostic, None, "The backed-up mouse is not connected."),
            Err(message) => return manual_response(&mut diagnostic, None, &message),
        };
        if let Err(message) = set_dpi(&target.control, backup.dpi) {
            return manual_response(
                &mut diagnostic,
                None,
                &format!("Restore DPI write failed: {message}"),
            );
        }
        if let Some(rate) = backup.polling_rate_hz {
            if let Err(message) = set_polling(&target.control, rate) {
                return manual_response(
                    &mut diagnostic,
                    None,
                    &format!("Restore polling-rate write failed: {message}"),
                );
            }
        }
        match read_settings(&target.control) {
            Ok((dpi, polling))
                if dpi == backup.dpi
                    && (backup.polling_rate_hz.is_none() || polling == backup.polling_rate_hz) =>
            {
                diagnostic.current_dpi = Some(dpi);
                diagnostic.current_polling_rate_hz = polling;
                diagnostic.restore_result = Some("Original settings restored and verified.".into());
                diagnostic.verification_result = diagnostic.restore_result.clone();
                remember(&diagnostic);
                response(
                    "success",
                    "Pre-Game Passport mouse settings were restored and verified.",
                    vec![
                        format!("DPI: {dpi}"),
                        polling
                            .map(|value| format!("Polling rate: {value} Hz"))
                            .unwrap_or_else(|| {
                                "Polling rate was not present in the backup.".into()
                            }),
                    ],
                    Some(diagnostic),
                    None,
                    None,
                )
            }
            Ok((dpi, polling)) => manual_response(
                &mut diagnostic,
                None,
                &format!("Restore readback mismatch (DPI {dpi}, polling {polling:?})."),
            ),
            Err(message) => manual_response(
                &mut diagnostic,
                None,
                &format!("Restore readback failed: {message}"),
            ),
        }
    }

    fn inspect() -> Result<(MouseDiagnostics, Result<Option<Target>, String>), String> {
        let api =
            HidApi::new().map_err(|error| format!("Windows HID enumeration failed: {error}"))?;
        let endpoints: Vec<Endpoint> = api.device_list().map(endpoint_from_info).collect();
        let mut devices: BTreeMap<String, MouseDeviceDiagnostics> = BTreeMap::new();
        for endpoint in &endpoints {
            if !is_mouse_like(endpoint) {
                continue;
            }
            devices
                .entry(endpoint.physical_key.clone())
                .or_insert_with(|| unsupported_diagnostic(endpoint));
        }
        let mut targets = Vec::new();
        let mut target_keys = HashSet::new();
        let mut probe_errors = Vec::new();
        for endpoint in &endpoints {
            if classify_vid_pid(endpoint.vendor_id, endpoint.product_id)
                == MouseAdapterKind::LogitechProbe
            {
                let mut found = false;
                let mut last_error = None;
                for index in [0xFF, 1, 2, 3, 4, 5, 6, 0] {
                    match probe_logitech(&api, endpoint, index) {
                        Ok(target) => {
                            let key = format!("{}:{index}", endpoint.physical_key);
                            if target_keys.insert(key) {
                                devices.insert(
                                    endpoint.physical_key.clone(),
                                    target.diagnostics.clone(),
                                );
                                targets.push(target);
                            }
                            found = true;
                            break;
                        }
                        Err(error) => last_error = Some(error),
                    }
                }
                if !found && probe_errors.len() < 12 {
                    probe_errors.push(format!(
                        "Logitech {} (VID {:04X}, PID {:04X}, interface {}): {}",
                        endpoint.product,
                        endpoint.vendor_id,
                        endpoint.product_id,
                        endpoint.interface_number,
                        last_error.unwrap_or_else(|| "HID++ Adjustable DPI недоступен".into())
                    ));
                }
            } else if classify_vid_pid(endpoint.vendor_id, endpoint.product_id)
                == MouseAdapterKind::Razer
            {
                if let Some(model) = RAZER_MODELS
                    .iter()
                    .find(|model| model.product_id == endpoint.product_id)
                    .copied()
                {
                    match probe_razer(&api, endpoint, model) {
                        Ok(target) => {
                            if target_keys.insert(endpoint.physical_key.clone()) {
                                devices.insert(
                                    endpoint.physical_key.clone(),
                                    target.diagnostics.clone(),
                                );
                                targets.push(target);
                            }
                        }
                        Err(error) if probe_errors.len() < 12 => probe_errors.push(format!(
                            "Razer {} (PID {:04X}): {error}",
                            endpoint.product, endpoint.product_id
                        )),
                        Err(_) => {}
                    }
                }
            }
        }
        let fixtures: Vec<SelectionFixture> = targets
            .iter()
            .map(|target| SelectionFixture {
                instance_id: target.diagnostics.instance_id.clone(),
                controllable: true,
            })
            .collect();
        let selection = select_unique_controllable(&fixtures)
            .map(|index| index.map(|index| targets[index].clone()));
        let mut diagnostics = MouseDiagnostics {
            devices: devices.into_values().collect(),
            probe_errors,
            selection_ambiguous: selection.is_err(),
            ..Default::default()
        };
        if let Ok(Some(target)) = &selection {
            diagnostics.selected_instance_id = Some(target.diagnostics.instance_id.clone());
            for device in &mut diagnostics.devices {
                device.selected = device.instance_id == target.diagnostics.instance_id;
            }
        }
        Ok((diagnostics, selection))
    }

    fn endpoint_from_info(info: &hidapi::DeviceInfo) -> Endpoint {
        let raw_path = info.path().to_string_lossy().to_string();
        let physical = info
            .serial_number()
            .filter(|value| !value.trim().is_empty())
            .map(|serial| {
                format!(
                    "{:04x}:{:04x}:serial:{serial}",
                    info.vendor_id(),
                    info.product_id()
                )
            })
            .unwrap_or_else(|| physical_key(&raw_path));
        Endpoint {
            path: info.path().to_owned(),
            vendor_id: info.vendor_id(),
            product_id: info.product_id(),
            manufacturer: info.manufacturer_string().unwrap_or("Unknown").to_string(),
            product: info
                .product_string()
                .unwrap_or("Unknown HID mouse")
                .to_string(),
            usage_page: info.usage_page(),
            usage: info.usage(),
            interface_number: info.interface_number(),
            instance_id: short_hash(&physical),
            physical_key: physical,
        }
    }

    fn physical_key(path: &str) -> String {
        let mut value = path.to_ascii_lowercase();
        if let Some(guid) = value.find("#{") {
            value.truncate(guid);
        }
        for marker in ["&col", "&mi_"] {
            while let Some(index) = value.find(marker) {
                let remove_to = (index + marker.len() + 2).min(value.len());
                value.replace_range(index..remove_to, "");
            }
        }
        value
    }

    fn short_hash(value: &str) -> String {
        let digest = Sha256::digest(value.as_bytes());
        digest[..8]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn is_mouse_like(endpoint: &Endpoint) -> bool {
        endpoint.vendor_id == LOGITECH_VID
            || endpoint.vendor_id == RAZER_VID
            || endpoint.vendor_id == LAMZU_VID
            || (endpoint.usage_page == 1 && endpoint.usage == 2)
    }

    fn unsupported_diagnostic(endpoint: &Endpoint) -> MouseDeviceDiagnostics {
        let (adapter, reason) = match endpoint.vendor_id {
            LOGITECH_VID => ("LogitechMouseAdapter", "This endpoint did not expose the required HID++ Adjustable DPI feature."),
            RAZER_VID => ("RazerMouseAdapter", "This Razer VID/PID is not in the implemented protocol matrix."),
            LAMZU_VID => ("LamzuMouseAdapter", "Detection is available, but Lamzu has no public, independently verifiable DPI/polling protocol."),
            _ => ("UnsupportedMouseAdapter", "No safe DPI/polling transport is implemented for this VID/PID."),
        };
        MouseDeviceDiagnostics {
            instance_id: endpoint.instance_id.clone(),
            vendor_id: format!("{:04X}", endpoint.vendor_id),
            product_id: format!("{:04X}", endpoint.product_id),
            manufacturer: endpoint.manufacturer.clone(),
            model: endpoint.product.clone(),
            connection: if endpoint.product.to_ascii_lowercase().contains("receiver")
                || endpoint.product.to_ascii_lowercase().contains("wireless")
            {
                "wireless receiver".into()
            } else {
                "USB/HID".into()
            },
            hid_usage: format!(
                "usage {:04X}:{:04X}, interface {}",
                endpoint.usage_page, endpoint.usage, endpoint.interface_number
            ),
            selected_adapter: adapter.into(),
            selected: false,
            capabilities: MouseCapabilities::unsupported(reason),
        }
    }

    fn probe_logitech(
        api: &HidApi,
        endpoint: &Endpoint,
        device_index: u8,
    ) -> Result<Target, String> {
        let device = api
            .open_path(&endpoint.path)
            .map_err(|error| error.to_string())?;
        let dpi_feature = hidpp_feature(&device, device_index, HIDPP_DPI)?;
        let rate_feature = hidpp_feature(&device, device_index, HIDPP_REPORT_RATE).ok();
        let dpi_values = hidpp_dpi_values(&device, device_index, dpi_feature)?;
        let (minimum, maximum, step) = summarize_dpi_values(&dpi_values);
        let rates = rate_feature
            .map(|feature| hidpp_rates(&device, device_index, feature).unwrap_or_default())
            .unwrap_or_default();
        let diagnostics = MouseDeviceDiagnostics {
            instance_id: endpoint.instance_id.clone(),
            vendor_id: format!("{:04X}", endpoint.vendor_id),
            product_id: format!("{:04X}", endpoint.product_id),
            manufacturer: if endpoint.manufacturer == "Unknown" {
                "Logitech".into()
            } else {
                endpoint.manufacturer.clone()
            },
            model: endpoint.product.clone(),
            connection: if device_index == 0xFF {
                "USB cable".into()
            } else {
                "Logitech receiver".into()
            },
            hid_usage: format!("HID++ device index {device_index:#04X}"),
            selected_adapter: "LogitechMouseAdapter (HID++ 2.0)".into(),
            selected: false,
            capabilities: MouseCapabilities {
                can_read_dpi: true,
                can_apply_dpi: true,
                can_read_polling_rate: rate_feature.is_some() && !rates.is_empty(),
                can_apply_polling_rate: rate_feature.is_some() && !rates.is_empty(),
                can_verify: true,
                dpi: Some(DpiCapabilities {
                    minimum,
                    maximum,
                    step,
                    values: dpi_values,
                }),
                polling_rates_hz: rates,
                reason: None,
            },
        };
        Ok(Target {
            diagnostics,
            control: Control::Logitech {
                path: endpoint.path.clone(),
                device_index,
                dpi_feature,
                rate_feature,
            },
        })
    }

    fn probe_razer(api: &HidApi, endpoint: &Endpoint, model: RazerModel) -> Result<Target, String> {
        let device = api
            .open_path(&endpoint.path)
            .map_err(|error| error.to_string())?;
        let _ = razer_get_dpi(&device)?;
        let diagnostics = MouseDeviceDiagnostics {
            instance_id: endpoint.instance_id.clone(),
            vendor_id: format!("{:04X}", endpoint.vendor_id),
            product_id: format!("{:04X}", endpoint.product_id),
            manufacturer: "Razer".into(),
            model: model.name.into(),
            connection: model.connection.into(),
            hid_usage: format!("feature-report interface {}", endpoint.interface_number),
            selected_adapter: "RazerMouseAdapter (HID feature reports)".into(),
            selected: false,
            capabilities: MouseCapabilities {
                can_read_dpi: true,
                can_apply_dpi: true,
                can_read_polling_rate: true,
                can_apply_polling_rate: true,
                can_verify: true,
                dpi: Some(DpiCapabilities {
                    minimum: 100,
                    maximum: model.maximum_dpi,
                    step: 50,
                    values: vec![],
                }),
                polling_rates_hz: model.polling_rates.to_vec(),
                reason: None,
            },
        };
        Ok(Target {
            diagnostics,
            control: Control::Razer {
                path: endpoint.path.clone(),
                model,
            },
        })
    }

    impl MouseHardwareAdapter for Control {
        fn read_current_settings(&self) -> Result<(u32, Option<u32>), String> {
            let api = HidApi::new().map_err(|error| error.to_string())?;
            match self {
                Control::Logitech {
                    path,
                    device_index,
                    dpi_feature,
                    rate_feature,
                } => {
                    let device = api.open_path(path).map_err(|error| error.to_string())?;
                    let dpi = hidpp_get_dpi(&device, *device_index, *dpi_feature)?;
                    let polling = rate_feature
                        .and_then(|feature| hidpp_get_rate(&device, *device_index, feature).ok());
                    Ok((dpi, polling))
                }
                Control::Razer { path, model } => {
                    let device = api.open_path(path).map_err(|error| error.to_string())?;
                    Ok((
                        razer_get_dpi(&device)?,
                        razer_get_polling(&device, *model).ok(),
                    ))
                }
            }
        }

        fn apply_dpi(&self, dpi: u32) -> Result<(), String> {
            let api = HidApi::new().map_err(|error| error.to_string())?;
            match self {
                Control::Logitech {
                    path,
                    device_index,
                    dpi_feature,
                    ..
                } => hidpp_set_dpi(
                    &api.open_path(path).map_err(|error| error.to_string())?,
                    *device_index,
                    *dpi_feature,
                    dpi,
                ),
                Control::Razer { path, .. } => razer_set_dpi(
                    &api.open_path(path).map_err(|error| error.to_string())?,
                    dpi,
                ),
            }
        }

        fn apply_polling_rate(&self, rate: u32) -> Result<(), String> {
            let api = HidApi::new().map_err(|error| error.to_string())?;
            match self {
                Control::Logitech {
                    path,
                    device_index,
                    rate_feature: Some(feature),
                    ..
                } => hidpp_set_rate(
                    &api.open_path(path).map_err(|error| error.to_string())?,
                    *device_index,
                    *feature,
                    rate,
                ),
                Control::Logitech {
                    rate_feature: None, ..
                } => Err("HID++ report-rate feature is unavailable.".into()),
                Control::Razer { path, model } => razer_set_polling(
                    &api.open_path(path).map_err(|error| error.to_string())?,
                    *model,
                    rate,
                ),
            }
        }
    }

    fn read_settings(control: &Control) -> Result<(u32, Option<u32>), String> {
        control.read_current_settings()
    }

    fn set_dpi(control: &Control, dpi: u32) -> Result<(), String> {
        control.apply_dpi(dpi)
    }

    fn set_polling(control: &Control, rate: u32) -> Result<(), String> {
        control.apply_polling_rate(rate)
    }

    fn hidpp_request(
        device: &HidDevice,
        device_index: u8,
        feature: u8,
        function: u8,
        params: &[u8],
    ) -> Result<Vec<u8>, String> {
        let mut report = [0u8; 20];
        report[0] = 0x11;
        report[1] = device_index;
        report[2] = feature;
        report[3] = (function << 4) | 0x0A;
        let count = params.len().min(16);
        report[4..4 + count].copy_from_slice(&params[..count]);
        device
            .write(&report)
            .map_err(|error| format!("HID++ write failed: {error}"))?;
        for _ in 0..8 {
            let mut answer = [0u8; 64];
            let count = device
                .read_timeout(&mut answer, 150)
                .map_err(|error| format!("HID++ read failed: {error}"))?;
            if count == 0 {
                continue;
            }
            if answer[0] == 0x8F || answer[0] == 0xFF {
                return Err(format!(
                    "HID++ device error 0x{:02X}.",
                    answer.get(4).copied().unwrap_or(0)
                ));
            }
            if count >= 4
                && answer[1] == device_index
                && answer[2] == feature
                && answer[3] == report[3]
            {
                return Ok(answer[..count].to_vec());
            }
        }
        Err("HID++ response timed out.".into())
    }

    fn hidpp_feature(device: &HidDevice, device_index: u8, feature_id: u16) -> Result<u8, String> {
        let response = hidpp_request(device, device_index, 0, 0, &feature_id.to_be_bytes())?;
        response
            .get(4)
            .copied()
            .filter(|index| *index != 0)
            .ok_or_else(|| format!("HID++ feature {feature_id:#06X} is unavailable."))
    }

    fn hidpp_dpi_values(device: &HidDevice, index: u8, feature: u8) -> Result<Vec<u32>, String> {
        let response = hidpp_request(device, index, feature, 1, &[0])?;
        let mut raw = Vec::new();
        for pair in response.get(4..).unwrap_or_default().chunks_exact(2) {
            let value = u16::from_be_bytes([pair[0], pair[1]]);
            if value == 0 {
                break;
            }
            raw.push(value);
        }
        if raw.is_empty() {
            return Err("HID++ returned an empty DPI capability list.".into());
        }
        let mut values = Vec::new();
        let mut step = None;
        for value in raw {
            if value > 0xE000 {
                step = Some((value - 0xE000) as u32);
            } else {
                values.push(value as u32);
            }
        }
        if let (Some(step), Some(first), Some(last)) =
            (step, values.first().copied(), values.last().copied())
        {
            values = (first..=last).step_by(step as usize).collect();
        }
        values.sort_unstable();
        values.dedup();
        Ok(values)
    }

    fn summarize_dpi_values(values: &[u32]) -> (u32, u32, u32) {
        let minimum = values.first().copied().unwrap_or(100);
        let maximum = values.last().copied().unwrap_or(minimum);
        let step = values
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .min()
            .unwrap_or(1)
            .max(1);
        (minimum, maximum, step)
    }

    fn hidpp_get_dpi(device: &HidDevice, index: u8, feature: u8) -> Result<u32, String> {
        let response = hidpp_request(device, index, feature, 2, &[0])?;
        if response.len() < 7 {
            return Err("HID++ DPI response was truncated.".into());
        }
        Ok(u16::from_be_bytes([response[5], response[6]]) as u32)
    }

    fn hidpp_set_dpi(device: &HidDevice, index: u8, feature: u8, dpi: u32) -> Result<(), String> {
        let dpi = u16::try_from(dpi).map_err(|_| "DPI exceeds HID++ range.".to_string())?;
        let bytes = dpi.to_be_bytes();
        let response = hidpp_request(device, index, feature, 3, &[0, bytes[0], bytes[1]])?;
        if response.len() < 7 || response[5] != bytes[0] || response[6] != bytes[1] {
            return Err("HID++ did not echo the requested DPI.".into());
        }
        Ok(())
    }

    fn hidpp_rates(device: &HidDevice, index: u8, feature: u8) -> Result<Vec<u32>, String> {
        let response = hidpp_request(device, index, feature, 0, &[])?;
        let flags = *response
            .get(4)
            .ok_or("HID++ report-rate response was truncated.")?;
        Ok([(1, 1000), (2, 500), (4, 250), (8, 125)]
            .into_iter()
            .filter_map(|(bit, rate)| (flags & bit != 0).then_some(rate))
            .collect())
    }

    fn hidpp_get_rate(device: &HidDevice, index: u8, feature: u8) -> Result<u32, String> {
        let response = hidpp_request(device, index, feature, 1, &[])?;
        interval_to_rate(
            *response
                .get(4)
                .ok_or("HID++ polling response was truncated.")?,
        )
    }

    fn hidpp_set_rate(device: &HidDevice, index: u8, feature: u8, rate: u32) -> Result<(), String> {
        let interval = rate_to_interval(rate)?;
        let response = hidpp_request(device, index, feature, 2, &[interval])?;
        if response.get(4).copied() != Some(interval) {
            return Err("HID++ did not echo the requested report interval.".into());
        }
        Ok(())
    }

    fn rate_to_interval(rate: u32) -> Result<u8, String> {
        match rate {
            1000 => Ok(1),
            500 => Ok(2),
            250 => Ok(4),
            125 => Ok(8),
            _ => Err(format!("{rate} Hz is not represented by HID++ 0x8060.")),
        }
    }
    fn interval_to_rate(interval: u8) -> Result<u32, String> {
        match interval {
            1 => Ok(1000),
            2 => Ok(500),
            4 => Ok(250),
            8 => Ok(125),
            _ => Err(format!("Unknown HID++ report interval {interval}.")),
        }
    }

    fn razer_exchange(
        device: &HidDevice,
        class: u8,
        command: u8,
        args: &[u8],
    ) -> Result<[u8; 90], String> {
        let mut report = [0u8; 90];
        report[1] = 0x1F;
        report[5] = args.len().min(80) as u8;
        report[6] = class;
        report[7] = command;
        report[8..8 + args.len().min(80)].copy_from_slice(&args[..args.len().min(80)]);
        report[88] = report[2..88].iter().fold(0u8, |crc, byte| crc ^ byte);
        device
            .send_feature_report(&report)
            .map_err(|error| format!("Razer feature write failed: {error}"))?;
        thread::sleep(Duration::from_millis(55));
        let mut response = [0u8; 90];
        let count = device
            .get_feature_report(&mut response)
            .map_err(|error| format!("Razer feature read failed: {error}"))?;
        if count < 89 {
            return Err("Razer feature response was truncated.".into());
        }
        let crc = response[2..88].iter().fold(0u8, |value, byte| value ^ byte);
        if crc != response[88] {
            return Err("Razer feature response failed CRC validation.".into());
        }
        if response[0] != 0x02 {
            return Err(format!(
                "Razer device returned status 0x{:02X}.",
                response[0]
            ));
        }
        if response[6] != class || response[7] != command {
            return Err("Razer response command mismatch.".into());
        }
        Ok(response)
    }

    fn razer_get_dpi(device: &HidDevice) -> Result<u32, String> {
        let response = razer_exchange(device, 0x04, 0x85, &[1, 0, 0, 0, 0, 0, 0])?;
        Ok(u16::from_be_bytes([response[9], response[10]]) as u32)
    }

    fn razer_set_dpi(device: &HidDevice, dpi: u32) -> Result<(), String> {
        let dpi =
            u16::try_from(dpi).map_err(|_| "DPI exceeds Razer protocol range.".to_string())?;
        let [high, low] = dpi.to_be_bytes();
        let _ = razer_exchange(device, 0x04, 0x05, &[1, high, low, high, low, 0, 0])?;
        Ok(())
    }

    fn razer_get_polling(device: &HidDevice, model: RazerModel) -> Result<u32, String> {
        let response = if model.polling2 {
            razer_exchange(device, 0x00, 0xC0, &[0, 0])?
        } else {
            razer_exchange(device, 0x00, 0x85, &[0])?
        };
        let code = if model.polling2 {
            response[9]
        } else {
            response[8]
        };
        razer_polling_from_code(code, model.polling2)
    }

    fn razer_set_polling(device: &HidDevice, model: RazerModel, rate: u32) -> Result<(), String> {
        let code = razer_polling_code(rate, model.polling2)?;
        if model.polling2 {
            let _ = razer_exchange(device, 0x00, 0x40, &[0, code])?;
        } else {
            let _ = razer_exchange(device, 0x00, 0x05, &[code])?;
        }
        Ok(())
    }

    fn razer_polling_code(rate: u32, modern: bool) -> Result<u8, String> {
        if modern {
            match rate {
                8000 => Ok(1),
                4000 => Ok(2),
                2000 => Ok(4),
                1000 => Ok(8),
                500 => Ok(0x10),
                250 => Ok(0x20),
                125 => Ok(0x40),
                _ => Err(format!("Unsupported Razer polling rate {rate}.")),
            }
        } else {
            match rate {
                1000 => Ok(1),
                500 => Ok(2),
                125 => Ok(8),
                _ => Err(format!("Unsupported legacy Razer polling rate {rate}.")),
            }
        }
    }

    fn razer_polling_from_code(code: u8, modern: bool) -> Result<u32, String> {
        if modern {
            match code {
                1 => Ok(8000),
                2 => Ok(4000),
                4 => Ok(2000),
                8 => Ok(1000),
                0x10 => Ok(500),
                0x20 => Ok(250),
                0x40 => Ok(125),
                _ => Err(format!("Unknown Razer polling code {code:#04X}.")),
            }
        } else {
            match code {
                1 => Ok(1000),
                2 => Ok(500),
                8 => Ok(125),
                _ => Err(format!("Unknown legacy Razer polling code {code:#04X}.")),
            }
        }
    }

    fn backup_dir() -> Result<PathBuf, String> {
        let root = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or("LOCALAPPDATA is unavailable.")?;
        Ok(root.join("Game Passport").join("Backups").join("Mouse"))
    }

    fn write_backup(
        instance_id: &str,
        dpi: u32,
        polling_rate_hz: Option<u32>,
    ) -> Result<String, String> {
        let dir = backup_dir()?;
        fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        let token = format!("mouse-{}.json", Utc::now().format("%Y%m%dT%H%M%S%3fZ"));
        let backup = MouseBackup {
            schema_version: 1,
            captured_at: Utc::now().to_rfc3339(),
            instance_id: instance_id.into(),
            dpi,
            polling_rate_hz,
        };
        fs::write(
            dir.join(&token),
            serde_json::to_vec_pretty(&backup).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        Ok(token)
    }

    fn read_backup(token: Option<&str>) -> Result<MouseBackup, String> {
        let dir = backup_dir()?;
        let path = match token {
            Some(value)
                if value.starts_with("mouse-")
                    && value.ends_with(".json")
                    && !value.contains(['/', '\\']) =>
            {
                dir.join(value)
            }
            Some(_) => return Err("Invalid Mouse backup token.".into()),
            None => newest_backup(&dir)?.ok_or("No pre-Game Passport Mouse backup exists.")?,
        };
        let backup: MouseBackup =
            serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
                .map_err(|_| "Mouse backup is corrupted.".to_string())?;
        if !validate_backup_values(backup.schema_version, backup.dpi, backup.polling_rate_hz) {
            return Err("Mouse backup is corrupted.".into());
        }
        Ok(backup)
    }

    fn newest_backup(dir: &Path) -> Result<Option<PathBuf>, String> {
        if !dir.exists() {
            return Ok(None);
        }
        let mut files: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .collect();
        files.sort();
        Ok(files.pop())
    }

    fn manual_response(
        diagnostic: &mut MouseDiagnostics,
        payload: Option<&MousePayload>,
        reason: &str,
    ) -> MouseCommandResponse {
        diagnostic.selection_ambiguous |= reason.contains("Multiple");
        diagnostic.verification_result = Some(reason.into());
        remember(diagnostic);
        let instruction = payload.map(|value| {
            format!(
                "Если автоматическое применение не сработало, выставьте в G HUB: {} DPI{}",
                value.dpi,
                value
                    .polling_rate_hz
                    .map(|rate| format!(" / {rate} Hz"))
                    .unwrap_or_default()
            )
        });
        let mut details: Vec<String> = instruction.into_iter().collect();
        details.push(reason.to_string());
        if diagnostic.devices.iter().any(|device| device.vendor_id == "046D") {
            details.push("Logitech G HUB управляет настройками мыши, но не гарантирует доступ к ним для другого приложения. Проверьте выбранный профиль/режим встроенной памяти G HUB.".into());
        }
        details.extend(diagnostic.probe_errors.iter().cloned());
        response(
            "warning",
            "Настройки мыши не применены автоматически.",
            details,
            Some(diagnostic.clone()),
            None,
            None,
        )
    }

    fn response(
        state: &str,
        message: &str,
        details: Vec<String>,
        diagnostics: Option<MouseDiagnostics>,
        payload: Option<MousePayload>,
        backup_token: Option<String>,
    ) -> MouseCommandResponse {
        MouseCommandResponse {
            state: state.into(),
            message: message.into(),
            details,
            retryable: matches!(state, "warning" | "error"),
            payload,
            diagnostics,
            backup_token,
        }
    }

    fn remember(diagnostics: &MouseDiagnostics) {
        if let Ok(mut last) = LAST_DIAGNOSTICS.get_or_init(|| Mutex::new(None)).lock() {
            *last = Some(diagnostics.clone());
        }
    }
    fn merge_last(current: &mut MouseDiagnostics) {
        if let Ok(last) = LAST_DIAGNOSTICS.get_or_init(|| Mutex::new(None)).lock() {
            if let Some(last) = last.as_ref() {
                if last.selected_instance_id == current.selected_instance_id {
                    current.requested_dpi = last.requested_dpi;
                    current.applied_dpi = last.applied_dpi;
                    current.requested_polling_rate_hz = last.requested_polling_rate_hz;
                    current.applied_polling_rate_hz = last.applied_polling_rate_hz;
                    current.verification_result = last.verification_result.clone();
                    current.backup_result = last.backup_result.clone();
                    current.restore_result = last.restore_result.clone();
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::{apply, capture, diagnostics, restore};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_dpi_is_preserved() {
        let caps = DpiCapabilities {
            minimum: 100,
            maximum: 3200,
            step: 50,
            values: vec![],
        };
        assert_eq!(normalize_desired_dpi(800, &caps), Some(800));
    }

    #[test]
    fn dpi_rounds_to_nearest_and_ties_go_lower() {
        let caps = DpiCapabilities {
            minimum: 100,
            maximum: 3200,
            step: 50,
            values: vec![],
        };
        assert_eq!(normalize_desired_dpi(805, &caps), Some(800));
        assert_eq!(normalize_desired_dpi(825, &caps), Some(800));
        assert_eq!(normalize_desired_dpi(826, &caps), Some(850));
    }

    #[test]
    fn dpi_is_clamped_to_hardware_range() {
        let caps = DpiCapabilities {
            minimum: 200,
            maximum: 1600,
            step: 100,
            values: vec![],
        };
        assert_eq!(normalize_desired_dpi(50, &caps), Some(200));
        assert_eq!(normalize_desired_dpi(5000, &caps), Some(1600));
    }

    #[test]
    fn discrete_dpi_capabilities_are_used() {
        let caps = DpiCapabilities {
            minimum: 400,
            maximum: 1600,
            step: 0,
            values: vec![400, 800, 1600],
        };
        assert_eq!(normalize_desired_dpi(1000, &caps), Some(800));
    }

    #[test]
    fn polling_falls_back_to_highest_not_above_request() {
        assert_eq!(normalize_polling_rate(2000, &[125, 500, 1000]), Some(1000));
        assert_eq!(
            normalize_polling_rate(4000, &[1000, 2000, 4000]),
            Some(4000)
        );
        assert_eq!(normalize_polling_rate(100, &[125, 500]), Some(125));
    }

    #[test]
    fn selection_rejects_multiple_physical_mice_but_deduplicates_interfaces() {
        let duplicate = vec![
            SelectionFixture {
                instance_id: "a".into(),
                controllable: true,
            },
            SelectionFixture {
                instance_id: "a".into(),
                controllable: true,
            },
        ];
        assert_eq!(select_unique_controllable(&duplicate), Ok(Some(0)));
        let multiple = vec![
            SelectionFixture {
                instance_id: "a".into(),
                controllable: true,
            },
            SelectionFixture {
                instance_id: "b".into(),
                controllable: true,
            },
        ];
        assert!(select_unique_controllable(&multiple).is_err());
    }

    #[test]
    fn unknown_devices_are_not_selected() {
        assert_eq!(
            select_unique_controllable(&[SelectionFixture {
                instance_id: "unknown".into(),
                controllable: false
            }]),
            Ok(None)
        );
    }

    #[test]
    fn vid_pid_detection_selects_only_implemented_families() {
        assert_eq!(
            classify_vid_pid(0x046D, 0xC548),
            MouseAdapterKind::LogitechProbe
        );
        assert_eq!(classify_vid_pid(0x1532, 0x00C1), MouseAdapterKind::Razer);
        assert_eq!(
            classify_vid_pid(0x1532, 0xFFFF),
            MouseAdapterKind::Unsupported
        );
        assert_eq!(
            classify_vid_pid(0x373E, 0x001E),
            MouseAdapterKind::LamzuDetectionOnly
        );
        assert_eq!(
            classify_vid_pid(0x1234, 0x5678),
            MouseAdapterKind::Unsupported
        );
    }

    #[test]
    fn unsupported_and_partial_results_never_claim_success() {
        assert_eq!(apply_outcome_state(false, true, false, false), "warning");
        assert_eq!(apply_outcome_state(true, true, false, false), "warning");
        assert_eq!(apply_outcome_state(true, true, true, true), "warning");
        assert_eq!(apply_outcome_state(true, true, true, false), "success");
    }

    #[test]
    fn corrupted_backup_values_are_rejected() {
        assert!(validate_backup_values(1, 800, Some(1000)));
        assert!(!validate_backup_values(2, 800, Some(1000)));
        assert!(!validate_backup_values(1, 0, Some(1000)));
        assert!(!validate_backup_values(1, 800, Some(16_000)));
    }

    #[test]
    fn corrupted_snapshot_is_rejected() {
        let invalid = MousePayload {
            schema_version: 99,
            captured_at: "now".into(),
            dpi: 800,
            polling_rate_hz: Some(1000),
        };
        assert!(validate_payload(&invalid).is_err());
        let invalid_rate = MousePayload {
            schema_version: 1,
            captured_at: "now".into(),
            dpi: 800,
            polling_rate_hz: Some(16_000),
        };
        assert!(validate_payload(&invalid_rate).is_err());
    }

    #[derive(Clone)]
    struct FixtureMouse {
        dpi: u32,
        polling: Option<u32>,
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct FixtureBackup {
        dpi: u32,
        polling: Option<u32>,
    }

    fn fixture_backup(mouse: &FixtureMouse) -> FixtureBackup {
        FixtureBackup {
            dpi: mouse.dpi,
            polling: mouse.polling,
        }
    }

    fn fixture_restore(mouse: &mut FixtureMouse, backup: &FixtureBackup) {
        mouse.dpi = backup.dpi;
        mouse.polling = backup.polling;
    }

    #[test]
    fn backup_fixture_captures_both_readable_values() {
        let mouse = FixtureMouse {
            dpi: 400,
            polling: Some(1000),
        };
        assert_eq!(
            fixture_backup(&mouse),
            FixtureBackup {
                dpi: 400,
                polling: Some(1000)
            }
        );
    }

    #[test]
    fn restore_fixture_returns_pre_apply_values() {
        let mut mouse = FixtureMouse {
            dpi: 400,
            polling: Some(1000),
        };
        let backup = fixture_backup(&mouse);
        mouse.dpi = 800;
        mouse.polling = Some(2000);
        fixture_restore(&mut mouse, &backup);
        assert_eq!(mouse.dpi, 400);
        assert_eq!(mouse.polling, Some(1000));
    }
}
