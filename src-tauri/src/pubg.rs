use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA_VERSION: u32 = 1;
const MAX_FILES: usize = 3;
const MAX_ENTRIES: usize = 8192;
const MAX_VALUE_BYTES: usize = 256 * 1024;
const MAX_SNAPSHOT_BYTES: usize = 2 * 1024 * 1024;
const ALLOWED_FILES: [&str; 3] = ["GameUserSettings.ini", "Input.ini", "Scalability.ini"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PubgIniEntry {
    pub section: String,
    pub key: String,
    #[serde(default)]
    pub operator: String,
    pub value: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PubgConfigFile {
    pub relative_id: String,
    pub entries: Vec<PubgIniEntry>,
    pub sha256: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PubgCategorySummary {
    pub gameplay: usize,
    pub keybinds: usize,
    pub graphics: usize,
    pub audio: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PubgNormalizedSettings {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub display_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PubgPayload {
    pub schema_version: u32,
    pub captured_at: String,
    pub game: String,
    pub files: Vec<PubgConfigFile>,
    pub normalized: PubgNormalizedSettings,
    pub captured_categories: PubgCategorySummary,
    pub unsupported_categories: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PubgDiagnostics {
    pub pubg_detected: bool,
    pub install_path: Option<String>,
    pub config_directory: Option<String>,
    pub config_files_found: Vec<String>,
    pub process_running: bool,
    pub capture_result: Option<String>,
    pub apply_result: Option<String>,
    pub backup_result: Option<String>,
    pub restore_result: Option<String>,
    pub categories: PubgCategorySummary,
    pub unsupported_settings: Vec<String>,
    pub parse_errors: Vec<String>,
    pub write_errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PubgCommandResponse {
    state: String,
    message: String,
    details: Vec<String>,
    retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<PubgPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<PubgDiagnostics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_token: Option<String>,
}

impl PubgCommandResponse {
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
    fn error(
        message: impl Into<String>,
        details: Vec<String>,
        retryable: bool,
        diagnostics: Option<PubgDiagnostics>,
    ) -> Self {
        Self {
            state: "error".into(),
            message: message.into(),
            details,
            retryable,
            payload: None,
            diagnostics,
            backup_token: None,
        }
    }
}

fn safe_file_id(value: &str) -> bool {
    ALLOWED_FILES
        .iter()
        .any(|name| name.eq_ignore_ascii_case(value))
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains("..")
}

fn forbidden_entry(section: &str, key: &str, _value: &str) -> bool {
    let candidate = format!("{section} {key}").to_ascii_lowercase();
    [
        "password",
        "authtoken",
        "auth_token",
        "cookie",
        "sessiontoken",
        "session_token",
        "steamlogin",
        "steam_login",
        "accountid",
        "userid",
        "machineid",
        "deviceid",
        "monitorid",
        "monitorname",
        "gpuadapter",
        "graphicsadapter",
        "audiooutputdevice",
        "audioinputdevice",
        "voicedevice",
        "preferredmonitor",
        "windowpos",
        "benchmarkresult",
        "lastrecommended",
        "battleye",
        "displayfrequency",
        "refreshrate",
    ]
    .iter()
    .any(|marker| candidate.contains(marker))
}

fn sensitive_identifier(identifier: &str) -> bool {
    let value = identifier.to_ascii_lowercase();
    [
        "password",
        "authtoken",
        "auth_token",
        "cookie",
        "sessiontoken",
        "session_token",
        "steamlogin",
        "steam_login",
        "accountid",
        "userid",
        "machineid",
        "deviceid",
        "monitorid",
        "monitorname",
        "gpuadapter",
        "graphicsadapter",
        "audiooutputdevice",
        "audioinputdevice",
        "voicedevice",
        "preferredmonitor",
        "windowpos",
        "benchmarkresult",
        "lastrecommended",
        "battleye",
        "displayfrequency",
        "refreshrate",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

// PUBG stores many preferences in one nested Unreal struct. Redact unsafe fields in-place
// instead of dropping the whole TslPersistantData value and losing sensitivities/binds.
fn redact_structured_value(value: &str) -> (String, usize) {
    let bytes = value.as_bytes();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;
    let mut redacted = 0;
    while cursor < bytes.len() {
        let start = cursor;
        if !bytes[cursor].is_ascii() {
            let character = value[cursor..].chars().next().expect("valid UTF-8");
            output.push(character);
            cursor += character.len_utf8();
            continue;
        }
        if !(bytes[cursor].is_ascii_alphabetic() || bytes[cursor] == b'_') {
            output.push(bytes[cursor] as char);
            cursor += 1;
            continue;
        }
        cursor += 1;
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_alphanumeric() || matches!(bytes[cursor], b'_' | b'.'))
        {
            cursor += 1;
        }
        let identifier = &value[start..cursor];
        let mut equals = cursor;
        while equals < bytes.len() && bytes[equals].is_ascii_whitespace() {
            equals += 1;
        }
        if equals >= bytes.len() || bytes[equals] != b'=' || !sensitive_identifier(identifier) {
            output.push_str(&value[start..cursor]);
            continue;
        }
        output.push_str(identifier);
        output.push_str("=\"\"");
        cursor = equals + 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let mut depth = 0i32;
        let mut quoted = false;
        let mut escaped = false;
        while cursor < bytes.len() {
            let character = bytes[cursor];
            if quoted {
                if escaped {
                    escaped = false;
                } else if character == b'\\' {
                    escaped = true;
                } else if character == b'"' {
                    quoted = false;
                }
            } else if character == b'"' {
                quoted = true;
            } else if character == b'(' {
                depth += 1;
            } else if character == b')' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            } else if character == b',' && depth == 0 {
                break;
            }
            cursor += 1;
        }
        redacted += 1;
    }
    (output, redacted)
}

fn categories_for(file: &str, section: &str, key: &str, value: &str) -> Vec<String> {
    let text = format!("{section} {key} {value}").to_ascii_lowercase();
    let mut out = BTreeSet::new();
    let add = |out: &mut BTreeSet<String>, name: &str| {
        out.insert(name.to_string());
    };
    if file.eq_ignore_ascii_case("Input.ini")
        || [
            "key",
            "bind",
            "input",
            "actionmapping",
            "axismapping",
            "custominputsettings",
        ]
        .iter()
        .any(|x| text.contains(x))
    {
        add(&mut out, "keybinds");
    }
    if file.eq_ignore_ascii_case("Scalability.ini")
        || [
            "resolution",
            "fullscreen",
            "windowmode",
            "screen",
            "render",
            "quality",
            "scalability",
            "shadow",
            "texture",
            "foliage",
            "effect",
            "postprocess",
            "antialias",
            "vsync",
            "motionblur",
            "sharpen",
            "framerate",
            "fps",
            "directx",
            "viewdistance",
            "sg.",
        ]
        .iter()
        .any(|x| text.contains(x))
    {
        add(&mut out, "graphics");
    }
    if [
        "audio",
        "sound",
        "volume",
        "voice",
        "music",
        "mastervolume",
        "effectvolume",
        "uivolume",
        "mic",
    ]
    .iter()
    .any(|x| text.contains(x))
    {
        add(&mut out, "audio");
    }
    if [
        "sensitive",
        "sensitivity",
        "mouse",
        "fov",
        "gameplay",
        "toggle",
        "hold",
        "lean",
        "scope",
        "ads",
        "aim",
        "tslpersistantdata",
    ]
    .iter()
    .any(|x| text.contains(x))
    {
        add(&mut out, "gameplay");
    }
    if out.is_empty() && file.eq_ignore_ascii_case("GameUserSettings.ini") {
        add(&mut out, "gameplay");
    }
    out.into_iter().collect()
}

fn entry_allowed(file: &str, section: &str, key: &str, value: &str) -> bool {
    if forbidden_entry(section, key, value) {
        return false;
    }
    if file.eq_ignore_ascii_case("GameUserSettings.ini") {
        let section = section.to_ascii_lowercase();
        return section.contains("tslgameusersettings")
            || section.contains("gameusersettings")
            || section.contains("scalabilitygroups");
    }
    if file.eq_ignore_ascii_case("Input.ini") {
        return section.to_ascii_lowercase().contains("input");
    }
    file.eq_ignore_ascii_case("Scalability.ini")
        && (section.to_ascii_lowercase().contains("scalability")
            || key.to_ascii_lowercase().starts_with("sg."))
}

pub fn parse_portable_ini(file: &str, text: &str) -> Result<(Vec<PubgIniEntry>, usize), String> {
    if !safe_file_id(file) {
        return Err("Unsafe PUBG configuration file identifier.".into());
    }
    if text.contains('\0') {
        return Err(format!("{file} contains a NUL byte."));
    }
    let mut section = String::new();
    let mut entries = Vec::new();
    let mut excluded = 0;
    for (index, raw) in text.lines().enumerate() {
        if raw.len() > MAX_VALUE_BYTES {
            return Err(format!(
                "{file} line {} exceeds the safe size limit.",
                index + 1
            ));
        }
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            if section.is_empty() || section.len() > 512 {
                return Err(format!(
                    "Malformed section in {file} at line {}.",
                    index + 1
                ));
            }
            continue;
        }
        let Some((raw_key, value)) = line.split_once('=') else {
            continue;
        };
        let raw_key = raw_key.trim();
        let (operator, key) = match raw_key.chars().next() {
            Some(character @ ('+' | '-' | '!' | '.')) => (
                character.to_string(),
                raw_key[character.len_utf8()..].trim(),
            ),
            _ => (String::new(), raw_key),
        };
        if section.is_empty() || key.is_empty() || key.len() > 512 {
            return Err(format!("Malformed key in {file} at line {}.", index + 1));
        }
        let (value, redacted) = redact_structured_value(value.trim());
        excluded += redacted;
        if !entry_allowed(file, &section, key, &value) {
            excluded += 1;
            continue;
        }
        entries.push(PubgIniEntry {
            section: section.clone(),
            key: key.into(),
            operator,
            value: value.clone(),
            categories: categories_for(file, &section, key, &value),
        });
        if entries.len() > MAX_ENTRIES {
            return Err(format!("{file} contains too many portable settings."));
        }
    }
    Ok((entries, excluded))
}

fn canonical_entries(entries: &[PubgIniEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            format!(
                "[{}]\n{}{}={}\n",
                entry.section, entry.operator, entry.key, entry.value
            )
        })
        .collect()
}

fn file_hash(entries: &[PubgIniEntry]) -> String {
    format!(
        "{:x}",
        Sha256::digest(canonical_entries(entries).as_bytes())
    )
}

pub fn summarize(files: &[PubgConfigFile]) -> PubgCategorySummary {
    let mut result = PubgCategorySummary::default();
    for category in files
        .iter()
        .flat_map(|file| file.entries.iter())
        .flat_map(|entry| &entry.categories)
    {
        match category.as_str() {
            "gameplay" => result.gameplay += 1,
            "keybinds" => result.keybinds += 1,
            "graphics" => result.graphics += 1,
            "audio" => result.audio += 1,
            _ => {}
        }
    }
    result
}

fn normalized(files: &[PubgConfigFile]) -> PubgNormalizedSettings {
    let entries = files
        .iter()
        .find(|file| {
            file.relative_id
                .eq_ignore_ascii_case("GameUserSettings.ini")
        })
        .map(|file| file.entries.as_slice())
        .unwrap_or_default();
    let number = |name: &str| {
        entries
            .iter()
            .rev()
            .find(|entry| entry.key.eq_ignore_ascii_case(name))
            .and_then(|entry| entry.value.trim_matches('"').parse().ok())
    };
    let mode = number("FullscreenMode").or_else(|| number("LastConfirmedFullscreenMode"));
    PubgNormalizedSettings {
        width: number("ResolutionSizeX"),
        height: number("ResolutionSizeY"),
        display_mode: match mode {
            Some(0) => "fullscreen",
            Some(1) => "borderless",
            Some(2) => "windowed",
            _ => "unknown",
        }
        .into(),
    }
}

pub fn validate_payload(payload: &PubgPayload) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION || payload.game != "pubg" {
        return Err("Unsupported or corrupted PUBG snapshot.".into());
    }
    if payload.files.is_empty() || payload.files.len() > MAX_FILES {
        return Err("PUBG snapshot has an invalid file count.".into());
    }
    let mut seen = BTreeSet::new();
    let mut count = 0;
    let mut total_bytes = 0;
    for file in &payload.files {
        if !safe_file_id(&file.relative_id) || !seen.insert(file.relative_id.to_ascii_lowercase()) {
            return Err("PUBG snapshot contains an unsafe or duplicate file identifier.".into());
        }
        count += file.entries.len();
        total_bytes += canonical_entries(&file.entries).len();
        if count > MAX_ENTRIES
            || total_bytes > MAX_SNAPSHOT_BYTES
            || file.entries.iter().any(|entry| {
                entry.section.is_empty()
                    || entry.key.is_empty()
                    || entry.value.len() > MAX_VALUE_BYTES
                    || forbidden_entry(&entry.section, &entry.key, &entry.value)
                    || redact_structured_value(&entry.value).1 > 0
                    || !entry_allowed(&file.relative_id, &entry.section, &entry.key, &entry.value)
            })
        {
            return Err("PUBG snapshot contains unsafe or malformed settings.".into());
        }
        if file.sha256 != file_hash(&file.entries) {
            return Err("PUBG snapshot integrity check failed.".into());
        }
    }
    Ok(())
}

fn is_pubg_process_name(name: &str) -> bool {
    [
        "TslGame.exe",
        "TslGame_BE.exe",
        "TslGame_UC.exe",
        "ExecPubg.exe",
    ]
    .iter()
    .any(|expected| expected.eq_ignore_ascii_case(name))
}

pub fn merge_ini(current: &str, saved: &[PubgIniEntry]) -> Result<String, String> {
    if current.contains('\0') {
        return Err("Target PUBG config contains a NUL byte.".into());
    }
    let keys = saved
        .iter()
        .map(|entry| {
            (
                entry.section.to_ascii_lowercase(),
                entry.key.to_ascii_lowercase(),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut section = String::new();
    let mut output = Vec::new();
    for raw in current.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            output.push(raw.to_string());
            continue;
        }
        if let Some((raw_key, _)) = line.split_once('=') {
            let key = raw_key
                .trim()
                .trim_start_matches(['+', '-', '!', '.'])
                .trim()
                .to_ascii_lowercase();
            if keys.contains(&(section.clone(), key)) {
                continue;
            }
        }
        output.push(raw.to_string());
    }
    let mut grouped: BTreeMap<&str, Vec<&PubgIniEntry>> = BTreeMap::new();
    for entry in saved {
        grouped.entry(&entry.section).or_default().push(entry);
    }
    output.push(String::new());
    output.push("; Game Passport portable PUBG settings".into());
    for (section, entries) in grouped {
        output.push(format!("[{section}]"));
        for entry in entries {
            output.push(format!("{}{}={}", entry.operator, entry.key, entry.value));
        }
    }
    Ok(format!("{}\r\n", output.join("\r\n")))
}

#[cfg(not(target_os = "windows"))]
pub fn capture() -> PubgCommandResponse {
    PubgCommandResponse::unsupported(
        "PUBG settings are supported only in the Windows desktop application.",
    )
}
#[cfg(not(target_os = "windows"))]
pub fn apply(_payload: PubgPayload) -> PubgCommandResponse {
    PubgCommandResponse::unsupported(
        "PUBG settings are supported only in the Windows desktop application.",
    )
}
#[cfg(not(target_os = "windows"))]
pub fn preflight() -> PubgCommandResponse {
    PubgCommandResponse::unsupported(
        "PUBG preflight is supported only in the Windows desktop application.",
    )
}
#[cfg(not(target_os = "windows"))]
pub fn restore(_backup_token: Option<String>) -> PubgCommandResponse {
    PubgCommandResponse::unsupported(
        "PUBG restore is supported only in the Windows desktop application.",
    )
}
#[cfg(not(target_os = "windows"))]
pub fn diagnostics() -> PubgCommandResponse {
    PubgCommandResponse::unsupported(
        "PUBG diagnostics are supported only in the Windows desktop application.",
    )
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use chrono::Utc;
    use std::{
        env,
        ffi::OsStr,
        fs,
        mem::{size_of, zeroed},
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };
    use winreg::{enums::*, RegKey};

    #[derive(Default, Clone)]
    struct LastResults {
        capture: Option<String>,
        apply: Option<String>,
        backup: Option<String>,
        restore: Option<String>,
        parse: Vec<String>,
        write: Vec<String>,
    }
    static LAST: OnceLock<Mutex<LastResults>> = OnceLock::new();
    fn last() -> &'static Mutex<LastResults> {
        LAST.get_or_init(|| Mutex::new(LastResults::default()))
    }

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct BackupFile {
        relative_id: String,
        existed: bool,
        content_base64: String,
        sha256: String,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Backup {
        schema_version: u32,
        captured_at: String,
        config_flavor: String,
        files: Vec<BackupFile>,
    }

    pub fn preflight() -> PubgCommandResponse {
        if game_running() {
            return PubgCommandResponse::error(
                "Закройте PUBG перед сохранением или применением профиля.",
                vec!["После закрытия нажмите «Повторить проверку».".into()],
                true,
                Some(current_diagnostics()),
            );
        }
        PubgCommandResponse {
            state: "success".into(),
            message: "PUBG is closed. Configuration files can be changed safely.".into(),
            details: vec![],
            retryable: false,
            payload: None,
            diagnostics: Some(current_diagnostics()),
            backup_token: None,
        }
    }

    pub fn capture() -> PubgCommandResponse {
        if game_running() {
            return preflight();
        }
        let directory = match config_directory() {
            Ok(value) => value,
            Err(message) => {
                return PubgCommandResponse::error(
                    message,
                    vec![],
                    true,
                    Some(current_diagnostics()),
                )
            }
        };
        let mut files = vec![];
        let mut warnings = vec![];
        let mut parse_errors = vec![];
        for name in ALLOWED_FILES {
            let path = directory.join(name);
            if !path.is_file() {
                if name == "GameUserSettings.ini" {
                    parse_errors.push(format!("Required file not found: {name}"));
                } else {
                    warnings.push(format!("Optional file not found: {name}"));
                }
                continue;
            }
            match fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| parse_portable_ini(name, &text))
            {
                Ok((entries, excluded)) => {
                    if excluded > 0 {
                        warnings.push(format!("{name}: excluded {excluded} hardware-specific or sensitive setting(s)."));
                    }
                    if entries.is_empty() {
                        warnings.push(format!("{name}: no portable settings were found."));
                    } else {
                        files.push(PubgConfigFile {
                            relative_id: name.into(),
                            sha256: file_hash(&entries),
                            entries,
                        });
                    }
                }
                Err(error) => parse_errors.push(error),
            }
        }
        if !parse_errors.is_empty()
            || !files
                .iter()
                .any(|file| file.relative_id == "GameUserSettings.ini")
        {
            if let Ok(mut state) = last().lock() {
                state.capture = Some("error".into());
                state.parse = parse_errors.clone();
            }
            return PubgCommandResponse::error(
                "PUBG configuration could not be captured safely.",
                parse_errors,
                true,
                Some(current_diagnostics()),
            );
        }
        let categories = summarize(&files);
        let normalized = normalized(&files);
        for (name, count) in [
            ("gameplay", categories.gameplay),
            ("keybinds", categories.keybinds),
            ("graphics", categories.graphics),
            ("audio", categories.audio),
        ] {
            if count == 0 {
                warnings.push(format!(
                    "No {name} settings were confirmed in the current config."
                ));
            }
        }
        if normalized.width.is_none() || normalized.height.is_none() {
            warnings.push(
                "Resolution was not found; Display Adapter capture will report Warning.".into(),
            );
        }
        let unsupported_categories = vec![
            "monitor/GPU/audio device identifiers".into(),
            "refresh rate (handled as MAX_AVAILABLE by Display Adapter)".into(),
            "Engine.ini and arbitrary console variables".into(),
            "Steam/BattlEye/authentication data".into(),
        ];
        let payload = PubgPayload {
            schema_version: SCHEMA_VERSION,
            captured_at: Utc::now().to_rfc3339(),
            game: "pubg".into(),
            files,
            normalized,
            captured_categories: categories.clone(),
            unsupported_categories: unsupported_categories.clone(),
            warnings: warnings.clone(),
        };
        if let Ok(mut state) = last().lock() {
            state.capture = Some(
                if warnings.is_empty() {
                    "success"
                } else {
                    "warning"
                }
                .into(),
            );
            state.parse.clear();
        }
        PubgCommandResponse {
            state: if warnings.is_empty() {
                "success"
            } else {
                "warning"
            }
            .into(),
            message: "PUBG profile captured from real Windows configuration files.".into(),
            details: category_details(&categories, &warnings),
            retryable: !warnings.is_empty(),
            payload: Some(payload),
            diagnostics: Some(current_diagnostics()),
            backup_token: None,
        }
    }

    pub fn apply(payload: PubgPayload) -> PubgCommandResponse {
        if game_running() {
            return preflight();
        }
        if let Err(message) = validate_payload(&payload) {
            return PubgCommandResponse::error(message, vec![], false, Some(current_diagnostics()));
        }
        let directory = match config_directory() {
            Ok(value) => value,
            Err(message) => {
                return PubgCommandResponse::error(
                    message,
                    vec![],
                    true,
                    Some(current_diagnostics()),
                )
            }
        };
        let backup_path = match write_backup(&directory) {
            Ok(value) => value,
            Err(message) => {
                return PubgCommandResponse::error(
                    message,
                    vec![],
                    true,
                    Some(current_diagnostics()),
                )
            }
        };
        if let Ok(mut state) = last().lock() {
            state.backup = Some(backup_path.display().to_string());
            state.write.clear();
        }
        let mut applied = 0;
        let mut errors = vec![];
        for file in &payload.files {
            let target = directory.join(&file.relative_id);
            let current = fs::read_to_string(&target).unwrap_or_default();
            let merged = match merge_ini(&current, &file.entries) {
                Ok(value) => value,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            if let Err(error) = atomic_write(&target, merged.as_bytes()) {
                errors.push(format!("{}: {error}", file.relative_id));
                continue;
            }
            match fs::read_to_string(&target)
                .map_err(|e| e.to_string())
                .and_then(|text| parse_portable_ini(&file.relative_id, &text))
            {
                Ok((entries, _)) if entries_multiset_contains(&entries, &file.entries) => {
                    applied += 1
                }
                Ok(_) => errors.push(format!(
                    "{} verification did not match the saved snapshot.",
                    file.relative_id
                )),
                Err(error) => {
                    errors.push(format!("{} verification failed: {error}", file.relative_id))
                }
            }
        }
        if !errors.is_empty() {
            let rollback = restore_path(&backup_path)
                .map(|_| "Automatic PUBG rollback completed.".to_string())
                .unwrap_or_else(|e| format!("Automatic PUBG rollback failed: {e}"));
            if let Ok(mut state) = last().lock() {
                state.apply = Some("error".into());
                state.write = errors.clone();
            }
            return PubgCommandResponse {
                state: "error".into(),
                message: "PUBG settings could not be applied completely; rollback was attempted."
                    .into(),
                details: errors.into_iter().chain([rollback]).collect(),
                retryable: true,
                payload: None,
                diagnostics: Some(current_diagnostics()),
                backup_token: Some(backup_path.display().to_string()),
            };
        }
        if let Ok(mut state) = last().lock() {
            state.apply = Some("success".into());
            state.write.clear();
        }
        PubgCommandResponse {
            state: "success".into(),
            message:
                "PUBG gameplay, keybind, graphics and audio settings were applied and verified."
                    .into(),
            details: vec![
                format!("Configuration files applied: {applied}"),
                format!("Local backup: {}", backup_path.display()),
            ],
            retryable: false,
            payload: None,
            diagnostics: Some(current_diagnostics()),
            backup_token: Some(backup_path.display().to_string()),
        }
    }

    pub fn restore(token: Option<String>) -> PubgCommandResponse {
        if game_running() {
            return preflight();
        }
        let path = match token
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(latest_backup)
        {
            Ok(value) => value,
            Err(message) => {
                return PubgCommandResponse::error(
                    message,
                    vec![],
                    false,
                    Some(current_diagnostics()),
                )
            }
        };
        match restore_path(&path) {
            Ok(count) => {
                if let Ok(mut state) = last().lock() {
                    state.restore = Some("success".into());
                }
                PubgCommandResponse {
                    state: "success".into(),
                    message: "Restored pre-Game Passport PUBG configuration.".into(),
                    details: vec![
                        format!("Files restored: {count}"),
                        format!("Backup used: {}", path.display()),
                    ],
                    retryable: false,
                    payload: None,
                    diagnostics: Some(current_diagnostics()),
                    backup_token: None,
                }
            }
            Err(message) => {
                if let Ok(mut state) = last().lock() {
                    state.restore = Some("error".into());
                    state.write.push(message.clone());
                }
                PubgCommandResponse::error(message, vec![], true, Some(current_diagnostics()))
            }
        }
    }

    pub fn diagnostics() -> PubgCommandResponse {
        let diagnostics = current_diagnostics();
        PubgCommandResponse {
            state: "success".into(),
            message: "PUBG diagnostics refreshed.".into(),
            details: vec![],
            retryable: false,
            payload: None,
            diagnostics: Some(diagnostics),
            backup_token: None,
        }
    }

    fn category_details(categories: &PubgCategorySummary, warnings: &[String]) -> Vec<String> {
        vec![
            format!("Gameplay ✓ ({} values)", categories.gameplay),
            format!(
                "Keybinds {} ({} values)",
                if categories.keybinds > 0 {
                    "✓"
                } else {
                    "Warning"
                },
                categories.keybinds
            ),
            format!(
                "Graphics {} ({} values)",
                if categories.graphics > 0 {
                    "✓"
                } else {
                    "Warning"
                },
                categories.graphics
            ),
            format!(
                "Audio {} ({} values)",
                if categories.audio > 0 {
                    "✓"
                } else {
                    "Warning"
                },
                categories.audio
            ),
        ]
        .into_iter()
        .chain(warnings.iter().cloned())
        .collect()
    }
    fn config_root() -> Result<PathBuf, String> {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("TslGame").join("Saved").join("Config"))
            .ok_or_else(|| {
                "LOCALAPPDATA is unavailable; PUBG configuration cannot be located.".into()
            })
    }
    fn config_directory() -> Result<PathBuf, String> {
        let root = config_root()?;
        for flavor in ["WindowsNoEditor", "Windows"] {
            let candidate = root.join(flavor);
            if candidate.join("GameUserSettings.ini").is_file() {
                return Ok(candidate);
            }
        }
        Err("PUBG settings were not found. Launch PUBG once, close it, and press Retry.".into())
    }
    fn backup_directory() -> Result<PathBuf, String> {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("Game Passport").join("Backups").join("PUBG"))
            .ok_or_else(|| "LOCALAPPDATA is unavailable; PUBG backup cannot be created.".into())
    }
    fn write_backup(directory: &Path) -> Result<PathBuf, String> {
        let root = backup_directory()?;
        fs::create_dir_all(&root)
            .map_err(|e| format!("Could not create PUBG backup folder: {e}"))?;
        let mut files = vec![];
        for name in ALLOWED_FILES {
            let path = directory.join(name);
            let bytes = if path.is_file() {
                fs::read(&path).map_err(|e| format!("Could not read {name} for backup: {e}"))?
            } else {
                vec![]
            };
            files.push(BackupFile {
                relative_id: name.into(),
                existed: path.is_file(),
                content_base64: BASE64.encode(&bytes),
                sha256: format!("{:x}", Sha256::digest(&bytes)),
            });
        }
        let flavor = directory
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| "Invalid PUBG configuration folder.".to_string())?
            .to_string();
        let backup = Backup {
            schema_version: 1,
            captured_at: Utc::now().to_rfc3339(),
            config_flavor: flavor,
            files,
        };
        let path = root.join(format!("{}.json", Utc::now().format("%Y%m%dT%H%M%S%.3fZ")));
        fs::write(
            &path,
            serde_json::to_vec_pretty(&backup).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("Could not write PUBG backup: {e}"))?;
        Ok(path)
    }
    fn latest_backup() -> Result<PathBuf, String> {
        let root = backup_directory()?;
        let mut paths = fs::read_dir(root)
            .map_err(|_| "No Game Passport PUBG backup was found.".to_string())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(OsStr::to_str) == Some("json"))
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .pop()
            .ok_or_else(|| "No Game Passport PUBG backup was found.".into())
    }
    fn restore_path(path: &Path) -> Result<usize, String> {
        let root = backup_directory()?
            .canonicalize()
            .map_err(|e| format!("PUBG backup folder is unavailable: {e}"))?;
        let path = path
            .canonicalize()
            .map_err(|e| format!("PUBG backup is unavailable: {e}"))?;
        if !path.starts_with(&root) {
            return Err("PUBG backup token points outside the Game Passport backup folder.".into());
        }
        let backup: Backup = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|_| "PUBG backup is corrupted.".to_string())?;
        if backup.schema_version != 1
            || !matches!(backup.config_flavor.as_str(), "WindowsNoEditor" | "Windows")
            || backup.files.len() != ALLOWED_FILES.len()
        {
            return Err("PUBG backup is corrupted or unsupported.".into());
        }
        let directory = config_root()?.join(&backup.config_flavor);
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let mut count = 0;
        for file in backup.files {
            if !safe_file_id(&file.relative_id) {
                return Err("PUBG backup contains an unsafe path.".into());
            }
            let bytes = BASE64
                .decode(file.content_base64)
                .map_err(|_| "PUBG backup contains invalid data.".to_string())?;
            if format!("{:x}", Sha256::digest(&bytes)) != file.sha256 {
                return Err("PUBG backup integrity check failed.".into());
            }
            let target = directory.join(file.relative_id);
            if file.existed {
                atomic_write(&target, &bytes)?;
                count += 1
            } else if target.exists() {
                fs::remove_file(target).map_err(|e| e.to_string())?;
            }
        }
        Ok(count)
    }
    fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?
        }
        let temp = path.with_extension("gamepassport.tmp");
        fs::write(&temp, bytes).map_err(|e| e.to_string())?;
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?
        }
        fs::rename(temp, path).map_err(|e| e.to_string())
    }
    fn entries_multiset_contains(actual: &[PubgIniEntry], expected: &[PubgIniEntry]) -> bool {
        let count = |items: &[PubgIniEntry]| {
            let mut map = BTreeMap::new();
            for e in items {
                *map.entry((
                    e.section.to_ascii_lowercase(),
                    e.key.to_ascii_lowercase(),
                    e.operator.clone(),
                    e.value.clone(),
                ))
                .or_insert(0usize) += 1;
            }
            map
        };
        let actual = count(actual);
        count(expected)
            .into_iter()
            .all(|(key, value)| actual.get(&key).copied().unwrap_or(0) >= value)
    }
    fn game_running() -> bool {
        [
            "TslGame.exe",
            "TslGame_BE.exe",
            "TslGame_UC.exe",
            "ExecPubg.exe",
        ]
        .iter()
        .any(|name| is_process_running(name))
    }
    fn is_process_running(expected: &str) -> bool {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return false;
            }
            let mut entry: PROCESSENTRY32W = zeroed();
            entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
            let mut found = false;
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    let end = entry
                        .szExeFile
                        .iter()
                        .position(|v| *v == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
                    if name.eq_ignore_ascii_case(expected) {
                        found = true;
                        break;
                    }
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
            found
        }
    }
    fn current_diagnostics() -> PubgDiagnostics {
        let directory = config_directory().ok();
        let install = find_install_path();
        let files = directory
            .as_ref()
            .map(|d| {
                ALLOWED_FILES
                    .iter()
                    .filter(|name| d.join(name).is_file())
                    .map(|name| name.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let parsed = directory
            .as_ref()
            .map(|d| {
                ALLOWED_FILES
                    .iter()
                    .filter_map(|name| {
                        fs::read_to_string(d.join(name))
                            .ok()
                            .and_then(|text| parse_portable_ini(name, &text).ok())
                            .map(|(entries, _)| PubgConfigFile {
                                relative_id: name.to_string(),
                                sha256: file_hash(&entries),
                                entries,
                            })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let categories = summarize(&parsed);
        let state = last().lock().map(|v| v.clone()).unwrap_or_default();
        PubgDiagnostics {
            pubg_detected: directory.is_some() || install.is_some(),
            install_path: install.map(|p| p.display().to_string()),
            config_directory: directory.map(|p| p.display().to_string()),
            config_files_found: files,
            process_running: game_running(),
            capture_result: state.capture,
            apply_result: state.apply,
            backup_result: state.backup,
            restore_result: state.restore,
            categories,
            unsupported_settings: vec![
                "Hardware device identifiers".into(),
                "Exact refresh rate".into(),
                "Engine.ini/custom cvars".into(),
            ],
            parse_errors: state.parse,
            write_errors: state.write,
        }
    }
    fn find_install_path() -> Option<PathBuf> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let steam = hkcu.open_subkey("Software\\Valve\\Steam").ok()?;
        let root = steam
            .get_value::<String, _>("SteamPath")
            .ok()
            .map(|p| PathBuf::from(p.replace('/', "\\")))?;
        let mut libraries = vec![root.clone()];
        if let Ok(text) = fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
            for line in text.lines() {
                let parts = line.split('"').collect::<Vec<_>>();
                if parts.len() >= 4 && parts[1].trim() == "path" {
                    libraries.push(PathBuf::from(parts[3].replace("\\\\", "\\")))
                }
            }
        }
        for library in libraries {
            let manifest = library.join("steamapps").join("appmanifest_578080.acf");
            if let Ok(text) = fs::read_to_string(manifest) {
                for line in text.lines() {
                    let parts = line.split('"').collect::<Vec<_>>();
                    if parts.len() >= 4 && parts[1].trim() == "installdir" {
                        let path = library
                            .join("steamapps")
                            .join("common")
                            .join(parts[3])
                            .join("TslGame")
                            .join("Binaries")
                            .join("Win64")
                            .join("TslGame.exe");
                        if path.is_file() {
                            return path.parent().map(Path::to_path_buf);
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(target_os = "windows")]
pub use windows::{apply, capture, diagnostics, preflight, restore};

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> &'static str {
        "[/Script/TslGame.TslGameUserSettings]\nResolutionSizeX=1728\nResolutionSizeY=1080\nFullscreenMode=0\nTslPersistantData=(MouseSensitiveList=(General=50.0),CustomInputSettings=((ActionName=\"Jump\",Key=SpaceBar)),MasterSoundVolume=80)\nAudioOutputDeviceId=secret-device\n[/Script/Engine.GameUserSettings]\nbUseVSync=False\n[ScalabilityGroups]\nsg.ShadowQuality=1\n"
    }
    fn payload() -> PubgPayload {
        let (entries, _) = parse_portable_ini("GameUserSettings.ini", fixture()).unwrap();
        let files = vec![PubgConfigFile {
            relative_id: "GameUserSettings.ini".into(),
            sha256: file_hash(&entries),
            entries,
        }];
        PubgPayload {
            schema_version: 1,
            captured_at: "2026-01-01T00:00:00Z".into(),
            game: "pubg".into(),
            normalized: normalized(&files),
            captured_categories: summarize(&files),
            files,
            unsupported_categories: vec![],
            warnings: vec![],
        }
    }
    #[test]
    fn parses_nested_persistent_data_without_splitting() {
        let (entries, excluded) = parse_portable_ini("GameUserSettings.ini", fixture()).unwrap();
        assert!(entries
            .iter()
            .any(|e| e.key == "TslPersistantData" && e.value.contains("ActionName")));
        assert_eq!(excluded, 1)
    }
    #[test]
    fn extracts_resolution_and_mode() {
        let p = payload();
        assert_eq!(p.normalized.width, Some(1728));
        assert_eq!(p.normalized.height, Some(1080));
        assert_eq!(p.normalized.display_mode, "fullscreen")
    }
    #[test]
    fn classifies_all_major_categories() {
        let p = payload();
        assert!(p.captured_categories.gameplay > 0);
        assert!(p.captured_categories.keybinds > 0);
        assert!(p.captured_categories.graphics > 0);
        assert!(p.captured_categories.audio > 0)
    }
    #[test]
    fn rejects_traversal_and_unknown_files() {
        let mut p = payload();
        p.files[0].relative_id = "../Engine.ini".into();
        assert!(validate_payload(&p).is_err())
    }
    #[test]
    fn rejects_integrity_mismatch() {
        let mut p = payload();
        p.files[0].entries[0].value = "999".into();
        assert!(validate_payload(&p).is_err())
    }
    #[test]
    fn rejects_secret_in_snapshot() {
        let mut p = payload();
        p.files[0].entries.push(PubgIniEntry {
            section: "/Script/TslGame.TslGameUserSettings".into(),
            key: "AuthToken".into(),
            operator: "".into(),
            value: "secret".into(),
            categories: vec!["gameplay".into()],
        });
        p.files[0].sha256 = file_hash(&p.files[0].entries);
        assert!(validate_payload(&p).is_err())
    }
    #[test]
    fn serializer_preserves_local_hardware_and_replaces_saved_keys() {
        let p = payload();
        let merged=merge_ini("[/Script/TslGame.TslGameUserSettings]\nResolutionSizeX=800\nAudioOutputDeviceId=local\n",&p.files[0].entries).unwrap();
        assert!(merged.contains("AudioOutputDeviceId=local"));
        assert!(!merged.contains("ResolutionSizeX=800"));
        assert!(merged.contains("ResolutionSizeX=1728"))
    }
    #[test]
    fn parses_array_operators() {
        let (text, _) = parse_portable_ini(
            "Input.ini",
            "[/Script/Engine.InputSettings]\n+ActionMappings=(ActionName=\"Jump\",Key=SpaceBar)\n",
        )
        .unwrap();
        assert_eq!(text[0].operator, "+")
    }
    #[test]
    fn malformed_nul_is_rejected() {
        assert!(parse_portable_ini("GameUserSettings.ini", "[x]\0\na=b").is_err())
    }
    #[test]
    fn missing_required_payload_is_rejected() {
        let mut p = payload();
        p.files.clear();
        assert!(validate_payload(&p).is_err())
    }
    #[test]
    fn schema_migration_boundary_is_explicit() {
        let mut p = payload();
        p.schema_version = 2;
        assert!(validate_payload(&p).is_err())
    }
    #[test]
    fn partial_category_summary_is_truthful() {
        let (entries, _) = parse_portable_ini(
            "Scalability.ini",
            "[ScalabilityGroups]\nsg.TextureQuality=2\n",
        )
        .unwrap();
        let files = vec![PubgConfigFile {
            relative_id: "Scalability.ini".into(),
            sha256: file_hash(&entries),
            entries,
        }];
        let s = summarize(&files);
        assert!(s.graphics > 0);
        assert_eq!(s.audio, 0)
    }
    #[test]
    fn file_hash_is_deterministic() {
        let p = payload();
        assert_eq!(p.files[0].sha256, file_hash(&p.files[0].entries))
    }

    #[test]
    fn recognizes_only_pubg_launch_processes() {
        assert!(is_pubg_process_name("TSLGAME.EXE"));
        assert!(is_pubg_process_name("TslGame_BE.exe"));
        assert!(!is_pubg_process_name("BEService_pubg.exe"));
        assert!(!is_pubg_process_name("steam.exe"));
    }

    #[test]
    fn redacts_nested_device_and_auth_values_without_losing_gameplay() {
        let text = "[/Script/TslGame.TslGameUserSettings]\nTslPersistantData=(MouseSensitiveList=(General=50),AudioOutputDeviceId=\"DEVICE-123\",AuthToken=\"secret\",FpsCameraFov=95)\n";
        let (entries, removed) = parse_portable_ini("GameUserSettings.ini", text).unwrap();
        assert_eq!(removed, 2);
        assert!(entries[0].value.contains("MouseSensitiveList"));
        assert!(entries[0].value.contains("FpsCameraFov=95"));
        assert!(!entries[0].value.contains("DEVICE-123"));
        assert!(!entries[0].value.contains("secret"));
    }
}
