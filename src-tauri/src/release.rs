use serde::{Deserialize, Serialize};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_LOG_BYTES: u64 = 1024 * 1024;
const MAX_LOG_FILES: usize = 3;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePreflight {
    pub app_version: String,
    pub build: String,
    pub windows_version: String,
    pub windows_supported: bool,
    pub steam_installed: bool,
    pub steam_user_available: bool,
    pub steam_path: Option<String>,
    pub cs2_installed: bool,
    pub pubg_config_available: bool,
    pub log_directory: Option<String>,
    pub administrator_required: bool,
    pub update_channel: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionLogEntry {
    timestamp: String,
    severity: String,
    adapter: String,
    operation: String,
    code: Option<String>,
    message: String,
    technical_details: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCommandResponse {
    pub state: String,
    pub message: String,
    pub path: Option<String>,
}

#[cfg(not(target_os = "windows"))]
pub fn preflight() -> ReleasePreflight {
    ReleasePreflight {
        app_version: APP_VERSION.into(),
        build: option_env!("GAME_PASSPORT_BUILD").unwrap_or("local").into(),
        windows_version: std::env::consts::OS.into(),
        windows_supported: false,
        steam_installed: false,
        steam_user_available: false,
        steam_path: None,
        cs2_installed: false,
        pubg_config_available: false,
        log_directory: None,
        administrator_required: false,
        update_channel: "Manual signed releases (updater not activated)".into(),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn append_log(_entry: ProductionLogEntry) -> FileCommandResponse {
    unsupported("Production file logging is available in the Windows desktop build.")
}

#[cfg(not(target_os = "windows"))]
pub fn save_report(_contents: String) -> FileCommandResponse {
    unsupported("Diagnostic report export is available in the Windows desktop build.")
}

#[cfg(not(target_os = "windows"))]
fn unsupported(message: &str) -> FileCommandResponse {
    FileCommandResponse {
        state: "unsupported".into(),
        message: message.into(),
        path: None,
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use chrono::Utc;
    use serde_json::Value;
    use std::{
        env,
        fs::{self, OpenOptions},
        io::Write,
        path::{Path, PathBuf},
    };
    use winreg::{enums::*, RegKey};

    pub fn preflight() -> ReleasePreflight {
        let steam = steam_root();
        let steam_user_available = active_steam_user();
        let cs2 = steam
            .as_deref()
            .map(|root| steam_app_installed(root, "730"))
            .unwrap_or(false);
        let pubg_config = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|root| {
                let config = root.join("TslGame").join("Saved").join("Config");
                config
                    .join("WindowsNoEditor")
                    .join("GameUserSettings.ini")
                    .is_file()
                    || config
                        .join("Windows")
                        .join("GameUserSettings.ini")
                        .is_file()
            })
            .unwrap_or(false);
        ReleasePreflight {
            app_version: APP_VERSION.into(),
            build: option_env!("GAME_PASSPORT_BUILD").unwrap_or("local").into(),
            windows_version: windows_version(),
            windows_supported: supported_windows(),
            steam_installed: steam.is_some(),
            steam_user_available,
            steam_path: steam.map(|path| path.display().to_string()),
            cs2_installed: cs2,
            pubg_config_available: pubg_config,
            log_directory: log_directory().ok().map(|path| path.display().to_string()),
            administrator_required: false,
            update_channel: "Manual signed releases (updater not activated)".into(),
        }
    }

    pub fn append_log(mut entry: ProductionLogEntry) -> FileCommandResponse {
        if !matches!(entry.severity.as_str(), "info" | "warning" | "error") {
            return failed("Invalid production log severity.");
        }
        entry.timestamp = clean(&entry.timestamp, 64);
        entry.adapter = clean(&entry.adapter, 80);
        entry.operation = clean(&entry.operation, 80);
        entry.code = entry.code.map(|value| clean(&value, 80));
        entry.message = clean(&entry.message, 1000);
        entry.technical_details = entry.technical_details.map(|value| clean(&value, 4000));
        let directory = match log_directory() {
            Ok(value) => value,
            Err(message) => return failed(&message),
        };
        if let Err(error) = fs::create_dir_all(&directory) {
            return failed(&format!("Could not create log directory: {error}"));
        }
        let path = directory.join("game-passport.log");
        if path
            .metadata()
            .map(|value| value.len() >= MAX_LOG_BYTES)
            .unwrap_or(false)
        {
            if let Err(error) = rotate_logs(&directory) {
                return failed(&format!("Could not rotate production logs: {error}"));
            }
        }
        let line = match serde_json::to_string(&entry) {
            Ok(value) => value,
            Err(error) => return failed(&format!("Could not serialize production log: {error}")),
        };
        let result = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| writeln!(file, "{line}"));
        match result {
            Ok(()) => FileCommandResponse {
                state: "success".into(),
                message: "Production log entry saved.".into(),
                path: Some(path.display().to_string()),
            },
            Err(error) => failed(&format!("Could not write production log: {error}")),
        }
    }

    pub fn save_report(contents: String) -> FileCommandResponse {
        if contents.len() > 2 * 1024 * 1024 {
            return failed("Diagnostic report exceeds the safe 2 MB limit.");
        }
        let mut value: Value = match serde_json::from_str(&contents) {
            Ok(value) => value,
            Err(_) => return failed("Diagnostic report is not valid JSON."),
        };
        sanitize_json(&mut value);
        let directory = match reports_directory() {
            Ok(value) => value,
            Err(message) => return failed(&message),
        };
        if let Err(error) = fs::create_dir_all(&directory) {
            return failed(&format!("Could not create report directory: {error}"));
        }
        let path = directory.join(format!(
            "Game-Passport-Diagnostics-{}.json",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        match serde_json::to_vec_pretty(&value)
            .map_err(std::io::Error::other)
            .and_then(|bytes| fs::write(&path, bytes))
        {
            Ok(()) => FileCommandResponse {
                state: "success".into(),
                message: "Diagnostic report saved.".into(),
                path: Some(path.display().to_string()),
            },
            Err(error) => failed(&format!("Could not save diagnostic report: {error}")),
        }
    }

    fn steam_root() -> Option<PathBuf> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey("Software\\Valve\\Steam")
            .ok()?
            .get_value::<String, _>("SteamPath")
            .ok()
            .map(|value| PathBuf::from(value.replace('/', "\\")))
            .filter(|path| path.is_dir())
    }

    fn active_steam_user() -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey("Software\\Valve\\Steam\\ActiveProcess")
            .ok()
            .and_then(|key| key.get_value::<u32, _>("ActiveUser").ok())
            .map(|value| value != 0)
            .unwrap_or(false)
    }

    fn steam_app_installed(steam: &Path, app_id: &str) -> bool {
        if steam
            .join("steamapps")
            .join(format!("appmanifest_{app_id}.acf"))
            .is_file()
        {
            return true;
        }
        let Ok(text) = fs::read_to_string(steam.join("steamapps").join("libraryfolders.vdf"))
        else {
            return false;
        };
        text.lines()
            .filter_map(|line| {
                let parts = line.split('"').collect::<Vec<_>>();
                (parts.len() >= 4 && parts[1].trim() == "path")
                    .then(|| PathBuf::from(parts[3].replace("\\\\", "\\")))
            })
            .any(|library| {
                library
                    .join("steamapps")
                    .join(format!("appmanifest_{app_id}.acf"))
                    .is_file()
            })
    }

    fn windows_version() -> String {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let Ok(key) = hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion") else {
            return "Windows (version unavailable)".into();
        };
        let product = key
            .get_value::<String, _>("ProductName")
            .unwrap_or_else(|_| "Windows".into());
        let display = key
            .get_value::<String, _>("DisplayVersion")
            .unwrap_or_default();
        let build = key
            .get_value::<String, _>("CurrentBuildNumber")
            .unwrap_or_default();
        format!("{product} {display} (build {build})").replace("  ", " ")
    }

    fn supported_windows() -> bool {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
            .ok()
            .and_then(|key| key.get_value::<String, _>("CurrentBuildNumber").ok())
            .and_then(|value| value.parse::<u32>().ok())
            .map(|build| build >= 19045)
            .unwrap_or(false)
    }

    fn local_app_data() -> Result<PathBuf, String> {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| "LOCALAPPDATA is unavailable.".into())
    }
    fn log_directory() -> Result<PathBuf, String> {
        Ok(local_app_data()?
            .join("app.gamepassport.desktop")
            .join("logs"))
    }
    fn reports_directory() -> Result<PathBuf, String> {
        env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|path| path.join("Documents").join("Game Passport Reports"))
            .ok_or_else(|| "Windows Documents folder is unavailable.".into())
    }

    fn rotate_logs(directory: &Path) -> std::io::Result<()> {
        let oldest = directory.join(format!("game-passport.log.{MAX_LOG_FILES}"));
        if oldest.exists() {
            fs::remove_file(oldest)?;
        }
        for index in (1..MAX_LOG_FILES).rev() {
            let from = directory.join(format!("game-passport.log.{index}"));
            let to = directory.join(format!("game-passport.log.{}", index + 1));
            if from.exists() {
                fs::rename(from, to)?;
            }
        }
        let current = directory.join("game-passport.log");
        if current.exists() {
            fs::rename(current, directory.join("game-passport.log.1"))?;
        }
        Ok(())
    }

    fn clean(value: &str, max: usize) -> String {
        let mut result = value.replace('\r', " ").replace('\0', "");
        if contains_secret_marker(&result) {
            result = "[REDACTED: sensitive-looking value]".into();
        }
        if let Some(home) = env::var_os("USERPROFILE") {
            result = result.replace(&home.to_string_lossy().to_string(), "%USERPROFILE%");
        }
        result.chars().take(max).collect()
    }

    fn contains_secret_marker(value: &str) -> bool {
        let lower = value.to_ascii_lowercase();
        [
            "authorization:",
            "bearer ",
            "access_token",
            "refresh_token",
            "password=",
            "cookie:",
            "supabase_anon_key",
            "steamloginsecure",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
    }

    fn sanitize_json(value: &mut Value) {
        match value {
            Value::Object(map) => {
                map.retain(|key, _| {
                    ![
                        "password",
                        "token",
                        "secret",
                        "cookie",
                        "authorization",
                        "email",
                        "userId",
                        "accessToken",
                        "refreshToken",
                    ]
                    .iter()
                    .any(|marker| {
                        key.to_ascii_lowercase()
                            .contains(&marker.to_ascii_lowercase())
                    })
                });
                for child in map.values_mut() {
                    sanitize_json(child);
                }
            }
            Value::Array(items) => {
                for child in items {
                    sanitize_json(child);
                }
            }
            Value::String(text) => {
                if contains_secret_marker(text)
                    || (text.starts_with("eyJ") && text.matches('.').count() == 2)
                {
                    *text = "[REDACTED]".into();
                } else if let Some(home) = env::var_os("USERPROFILE") {
                    *text = text.replace(&home.to_string_lossy().to_string(), "%USERPROFILE%");
                }
            }
            _ => {}
        }
    }

    fn failed(message: &str) -> FileCommandResponse {
        FileCommandResponse {
            state: "error".into(),
            message: message.into(),
            path: None,
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::{append_log, preflight, save_report};

#[cfg(test)]
mod tests {
    #[test]
    fn release_version_comes_from_package() {
        assert_eq!(super::APP_VERSION, "0.6.0");
    }
    #[test]
    fn log_limits_are_bounded() {
        assert_eq!(super::MAX_LOG_BYTES, 1024 * 1024);
        assert_eq!(super::MAX_LOG_FILES, 3);
    }
}
