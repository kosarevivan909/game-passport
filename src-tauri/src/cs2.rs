use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cs2ConfigFile {
    pub scope: String,
    pub relative_path: String,
    pub content_base64: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cs2Payload {
    pub schema_version: u32,
    pub captured_at: String,
    pub files: Vec<Cs2ConfigFile>,
    pub total_bytes: u64,
    pub core_files_found: Vec<String>,
    pub optional_files_missing: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cs2CommandResponse {
    state: String,
    message: String,
    details: Vec<String>,
    retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<Cs2Payload>,
}

impl Cs2CommandResponse {
    fn unsupported(message: &str) -> Self {
        Self {
            state: "unsupported".into(),
            message: message.into(),
            details: vec![],
            retryable: false,
            payload: None,
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn capture() -> Cs2CommandResponse {
    Cs2CommandResponse::unsupported(
        "CS2 settings are supported only in the Windows desktop application.",
    )
}

#[cfg(not(target_os = "windows"))]
pub fn apply(_payload: Cs2Payload) -> Cs2CommandResponse {
    Cs2CommandResponse::unsupported(
        "CS2 settings are supported only in the Windows desktop application.",
    )
}

#[cfg(not(target_os = "windows"))]
pub fn preflight() -> Cs2CommandResponse {
    Cs2CommandResponse::unsupported(
        "CS2 preflight is supported only in the Windows desktop application.",
    )
}

#[cfg(not(target_os = "windows"))]
pub fn restore() -> Cs2CommandResponse {
    Cs2CommandResponse::unsupported(
        "CS2 restore is supported only in the Windows desktop application.",
    )
}

#[cfg(target_os = "windows")]
mod windows {
    use super::{Cs2CommandResponse, Cs2ConfigFile, Cs2Payload};
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use chrono::Utc;
    use sha2::{Digest, Sha256};
    use std::{
        collections::{HashSet, VecDeque},
        env,
        ffi::OsStr,
        fs::{self, File},
        io::Write,
        mem::{size_of, zeroed},
        path::{Component, Path, PathBuf},
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };
    use winreg::{enums::*, RegKey};

    const APP_ID: &str = "730";
    const MAX_FILE_BYTES: u64 = 512 * 1024;
    const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024;
    const MAX_FILES: usize = 32;
    const REQUIRED_CORE: [&str; 2] = [
        "cs2_user_convars_0_slot0.vcfg",
        "cs2_user_keys_0_slot0.vcfg",
    ];
    const OPTIONAL_CORE: [&str; 2] = ["cs2_machine_convars.vcfg", "cs2_video.txt"];

    struct SteamContext {
        userdata_cfg: PathBuf,
        install_cfg: Option<PathBuf>,
    }

    #[derive(Debug)]
    struct UserFacingError {
        message: String,
        details: Vec<String>,
        retryable: bool,
    }

    impl UserFacingError {
        fn new(message: impl Into<String>, retryable: bool) -> Self {
            Self {
                message: message.into(),
                details: vec![],
                retryable,
            }
        }
        fn detail(mut self, detail: impl Into<String>) -> Self {
            self.details.push(detail.into());
            self
        }
        fn response(self) -> Cs2CommandResponse {
            Cs2CommandResponse {
                state: "error".into(),
                message: self.message,
                details: self.details,
                retryable: self.retryable,
                payload: None,
            }
        }
    }

    pub fn capture() -> Cs2CommandResponse {
        match capture_inner() {
            Ok(response) => response,
            Err(error) => error.response(),
        }
    }

    pub fn apply(payload: Cs2Payload) -> Cs2CommandResponse {
        match apply_inner(payload) {
            Ok(response) => response,
            Err(error) => error.response(),
        }
    }

    pub fn preflight() -> Cs2CommandResponse {
        if is_process_running("cs2.exe") {
            return Cs2CommandResponse {
                state: "error".into(),
                message: "Закройте Counter-Strike 2 перед настройкой компьютера.".into(),
                details: vec!["После закрытия нажмите «Повторить проверку».".into()],
                retryable: true,
                payload: None,
            };
        }
        Cs2CommandResponse {
            state: "success".into(),
            message: "Counter-Strike 2 is closed. System settings can be changed safely.".into(),
            details: vec![],
            retryable: false,
            payload: None,
        }
    }

    pub fn restore() -> Cs2CommandResponse {
        match restore_inner() {
            Ok(response) => response,
            Err(error) => error.response(),
        }
    }

    fn capture_inner() -> Result<Cs2CommandResponse, UserFacingError> {
        ensure_game_closed()?;
        let context = discover_steam_context()?;
        let mut files = Vec::new();
        let mut details = Vec::new();
        let mut core_files_found = Vec::new();
        let mut optional_files_missing = Vec::new();

        for name in REQUIRED_CORE.into_iter().chain(OPTIONAL_CORE) {
            let path = context.userdata_cfg.join(name);
            if path.is_file() {
                add_file(&mut files, "userdata", name, &path, false, &mut details)?;
                core_files_found.push(name.to_string());
            } else if REQUIRED_CORE.contains(&name) {
                return Err(UserFacingError::new(
                    "CS2 configuration is incomplete. Launch CS2 once, save your settings, close the game, and retry.",
                    true,
                ).detail(format!("Required file was not found: {name}")));
            } else {
                optional_files_missing.push(name.to_string());
            }
        }

        capture_userdata_cfg_files(&context.userdata_cfg, &mut files, &mut details)?;
        if let Some(install_cfg) = context.install_cfg.as_deref() {
            capture_autoexec_chain(install_cfg, &mut files, &mut details)?;
        } else {
            optional_files_missing.push("autoexec.cfg (CS2 installation not located)".into());
        }

        if files.len() > MAX_FILES {
            return Err(UserFacingError::new(
                "Too many CS2 configuration files were found. Nothing was saved.",
                false,
            ));
        }
        let total_bytes = files.iter().map(|file| file.size).sum::<u64>();
        if total_bytes > MAX_TOTAL_BYTES {
            return Err(UserFacingError::new(
                "CS2 configuration is larger than the safe 2 MB limit. Nothing was saved.",
                false,
            ));
        }

        let state = if optional_files_missing.is_empty() && details.is_empty() {
            "success"
        } else {
            "warning"
        };
        let message = if state == "success" {
            format!("Сохранено файлов настроек CS2: {}.", files.len())
        } else {
            format!("Сохранено файлов настроек CS2: {} (есть пояснения).", files.len())
        };
        let custom_count = files.len().saturating_sub(core_files_found.len());
        let video_captured = core_files_found.iter().any(|name| name == "cs2_video.txt");
        let payload = Cs2Payload {
            schema_version: 1,
            captured_at: Utc::now().to_rfc3339(),
            files,
            total_bytes,
            core_files_found,
            optional_files_missing: optional_files_missing.clone(),
        };
        let retryable = !optional_files_missing.is_empty();
        let mut summary = vec![
            "Сохранены чувствительность, игровые параметры и звук из cs2_user_convars_0_slot0.vcfg.".into(),
            "Сохранены привязки клавиш из cs2_user_keys_0_slot0.vcfg.".into(),
        ];
        if video_captured {
            summary.push("Сохранены переносимые параметры видео и разрешения из cs2_video.txt.".into());
        }
        if custom_count > 0 {
            summary.push(format!("Сохранено дополнительных CFG-файлов: {custom_count}."));
        }
        summary.push("Не сохраняются идентификаторы монитора, видеокарты и аудиоустройства; точная герцовка выбирается заново на целевом ПК.".into());
        summary.append(&mut details);
        details = summary;
        details.extend(
            optional_files_missing
                .iter()
                .map(|name| format!("Необязательный файл не найден: {name}")),
        );
        Ok(Cs2CommandResponse {
            state: state.into(),
            message,
            details,
            retryable,
            payload: Some(payload),
        })
    }

    fn apply_inner(payload: Cs2Payload) -> Result<Cs2CommandResponse, UserFacingError> {
        ensure_game_closed()?;
        validate_payload(&payload)?;
        let context = discover_steam_context()?;
        let needs_install = payload.files.iter().any(|file| file.scope == "install");
        if needs_install && context.install_cfg.is_none() {
            return Err(UserFacingError::new(
                "CS2 is not installed in a Steam Library on this computer.",
                true,
            ));
        }

        let decoded = decode_and_resolve(&payload, &context)?;
        let backup_root = backup_root()?;
        fs::create_dir_all(&backup_root)
            .map_err(|error| io_error("Could not create a CS2 backup folder.", error))?;

        let mut staged = Vec::new();
        for (index, item) in decoded.iter().enumerate() {
            match prepare_staged_file(index, item, &backup_root) {
                Ok(temporary) => staged.push((item, temporary)),
                Err(error) => {
                    cleanup_staged(&staged);
                    return Err(error.detail(format!(
                        "Any completed backup is available at: {}",
                        backup_root.display()
                    )));
                }
            }
        }

        if let Err(error) = ensure_game_closed() {
            cleanup_staged(&staged);
            return Err(error);
        }
        let mut committed: Vec<(&DecodedFile, Option<PathBuf>)> = Vec::new();
        for (index, (item, temporary)) in staged.iter().enumerate() {
            let previous = if item.target.exists() {
                let path = item
                    .target
                    .with_extension(format!("gamepassport-{index}.previous"));
                let _ = fs::remove_file(&path);
                if let Err(error) = fs::rename(&item.target, &path) {
                    rollback(&committed);
                    cleanup_staged(&staged);
                    return Err(io_error(
                        "Could not replace an existing CS2 configuration file.",
                        error,
                    )
                    .detail(format!("Backup is available at: {}", backup_root.display())));
                }
                Some(path)
            } else {
                None
            };
            if let Err(error) = fs::rename(temporary, &item.target) {
                if let Some(path) = previous.as_ref() {
                    let _ = fs::rename(path, &item.target);
                }
                rollback(&committed);
                cleanup_staged(&staged);
                return Err(
                    io_error("Could not activate a CS2 configuration file.", error)
                        .detail(format!("Backup is available at: {}", backup_root.display())),
                );
            }
            committed.push((item, previous));
        }
        for (item, _) in &committed {
            let written = match fs::read(&item.target) {
                Ok(bytes) => bytes,
                Err(error) => {
                    rollback(&committed);
                    return Err(io_error(
                        "Не удалось проверить записанный файл CS2; изменения отменены.",
                        error,
                    ));
                }
            };
            if written != item.content {
                rollback(&committed);
                return Err(UserFacingError::new(
                    "Проверка записанных настроек CS2 не пройдена; изменения отменены.",
                    true,
                )
                .detail(format!("Файл не совпал: {}", item.relative_path)));
            }
        }
        for (_, previous) in &committed {
            if let Some(path) = previous {
                let _ = fs::remove_file(path);
            }
        }

        let hardware_specific = payload.files.iter().any(|file| {
            matches!(
                file.relative_path.as_str(),
                "cs2_machine_convars.vcfg" | "cs2_video.txt"
            )
        });
        let mut details = vec![
            format!("Локальная резервная копия: {}", backup_root.display()),
            format!("Записано и проверено файлов: {}.", committed.len()),
        ];
        if hardware_specific {
            details.push("Разрешение и переносимые параметры видео записаны. После запуска CS2 может заменить значения, которые не поддерживаются этим компьютером.".into());
        }
        Ok(Cs2CommandResponse {
            state: if hardware_specific { "warning".into() } else { "success".into() },
            message: format!("Настройки CS2 применены и проверены: {} файлов. Запустите CS2 и проверьте итоговые значения в игре.", committed.len()),
            details,
            retryable: false,
            payload: None,
        })
    }

    fn restore_inner() -> Result<Cs2CommandResponse, UserFacingError> {
        ensure_game_closed()?;
        let context = discover_steam_context()?;
        let backups = cs2_backup_directory()?;
        let mut candidates = fs::read_dir(&backups)
            .map_err(|_| UserFacingError::new("No Game Passport CS2 backup was found.", false))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        candidates.sort();
        let backup = candidates
            .pop()
            .ok_or_else(|| UserFacingError::new("No Game Passport CS2 backup was found.", false))?;
        let mut restored = 0usize;
        let mut details = vec![format!("Backup used: {}", backup.display())];
        for scope in ["userdata", "install"] {
            let source_root = backup.join(scope);
            if !source_root.is_dir() {
                continue;
            }
            let target_root = match scope {
                "userdata" => context.userdata_cfg.as_path(),
                "install" => match context.install_cfg.as_deref() {
                    Some(value) => value,
                    None => {
                        details.push("Install-scoped CFG files could not be restored because the CS2 installation was not located.".into());
                        continue;
                    }
                },
                _ => unreachable!(),
            };
            let mut queue = VecDeque::from([source_root.clone()]);
            while let Some(directory) = queue.pop_front() {
                for entry in fs::read_dir(&directory)
                    .map_err(|error| io_error("Could not read the CS2 backup.", error))?
                {
                    let entry = entry
                        .map_err(|error| io_error("Could not read a CS2 backup entry.", error))?;
                    let source = entry.path();
                    if source.is_dir() {
                        queue.push_back(source);
                        continue;
                    }
                    let relative = source.strip_prefix(&source_root).map_err(|_| {
                        UserFacingError::new("CS2 backup contains an unsafe path.", false)
                    })?;
                    validate_file_name(scope, relative)?;
                    let target = target_root.join(relative);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).map_err(|error| {
                            io_error("Could not prepare the CS2 restore destination.", error)
                        })?;
                    }
                    let temporary = target.with_extension("gamepassport-restore.tmp");
                    fs::copy(&source, &temporary)
                        .map_err(|error| io_error("Could not stage a CS2 backup file.", error))?;
                    if target.exists() {
                        fs::remove_file(&target).map_err(|error| {
                            io_error("Could not replace a CS2 configuration file.", error)
                        })?;
                    }
                    fs::rename(&temporary, &target).map_err(|error| {
                        io_error("Could not activate a restored CS2 file.", error)
                    })?;
                    restored += 1;
                }
            }
        }
        if restored == 0 {
            return Err(UserFacingError::new(
                "The latest CS2 backup did not contain restorable files.",
                false,
            ));
        }
        details.push("Files created by a previous apply but absent from the backup are not deleted automatically.".into());
        Ok(Cs2CommandResponse {
            state: "warning".into(),
            message: format!("Restored {restored} CS2 configuration files."),
            details,
            retryable: false,
            payload: None,
        })
    }

    struct DecodedFile {
        scope: String,
        relative_path: String,
        target: PathBuf,
        content: Vec<u8>,
    }

    fn prepare_staged_file(
        index: usize,
        item: &DecodedFile,
        backup_root: &Path,
    ) -> Result<PathBuf, UserFacingError> {
        if let Some(parent) = item.target.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| io_error("Could not create a destination folder.", error))?;
        }
        let backup = backup_root.join(&item.scope).join(&item.relative_path);
        if item.target.is_file() {
            if let Some(parent) = backup.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| io_error("Could not prepare the backup folder.", error))?;
            }
            fs::copy(&item.target, &backup).map_err(|error| {
                io_error("Could not back up the existing CS2 configuration.", error)
            })?;
        }
        let temporary = item
            .target
            .with_extension(format!("gamepassport-{index}.tmp"));
        let mut output = File::create(&temporary)
            .map_err(|error| io_error("Could not stage a CS2 configuration file.", error))?;
        if let Err(error) = output
            .write_all(&item.content)
            .and_then(|_| output.sync_all())
        {
            drop(output);
            let _ = fs::remove_file(&temporary);
            return Err(io_error(
                "Could not write a staged CS2 configuration file.",
                error,
            ));
        }
        Ok(temporary)
    }

    fn decode_and_resolve(
        payload: &Cs2Payload,
        context: &SteamContext,
    ) -> Result<Vec<DecodedFile>, UserFacingError> {
        let mut result = Vec::new();
        let mut targets = HashSet::new();
        let mut computed_total = 0u64;
        for file in &payload.files {
            let relative = safe_relative_path(&file.relative_path)?;
            validate_file_name(&file.scope, &relative)?;
            let content = BASE64.decode(&file.content_base64).map_err(|_| {
                UserFacingError::new(
                    "The saved CS2 profile contains invalid encoded data.",
                    false,
                )
            })?;
            if content.len() as u64 != file.size || file.size > MAX_FILE_BYTES {
                return Err(UserFacingError::new(
                    "The saved CS2 profile failed its size validation.",
                    false,
                ));
            }
            let digest = format!("{:x}", Sha256::digest(&content));
            if digest != file.sha256.to_lowercase() {
                return Err(UserFacingError::new(
                    "The saved CS2 profile failed its integrity check. Nothing was changed.",
                    false,
                ));
            }
            if contains_sensitive_data(&content) {
                return Err(UserFacingError::new("The saved CS2 profile contains a blocked sensitive command. Nothing was changed.", false));
            }
            computed_total += file.size;
            let base = match file.scope.as_str() {
                "userdata" => &context.userdata_cfg,
                "install" => context.install_cfg.as_ref().ok_or_else(|| {
                    UserFacingError::new("CS2 installation folder is unavailable.", true)
                })?,
                _ => {
                    return Err(UserFacingError::new(
                        "The saved CS2 profile contains an unknown file scope.",
                        false,
                    ))
                }
            };
            let target = base.join(&relative);
            if !targets.insert(target.clone()) {
                return Err(UserFacingError::new(
                    "The saved CS2 profile contains duplicate files.",
                    false,
                ));
            }
            result.push(DecodedFile {
                scope: file.scope.clone(),
                relative_path: file.relative_path.clone(),
                target,
                content,
            });
        }
        if computed_total != payload.total_bytes || computed_total > MAX_TOTAL_BYTES {
            return Err(UserFacingError::new(
                "The saved CS2 profile failed its total-size validation.",
                false,
            ));
        }
        Ok(result)
    }

    fn validate_payload(payload: &Cs2Payload) -> Result<(), UserFacingError> {
        if payload.schema_version != 1 {
            return Err(UserFacingError::new(
                "This CS2 profile format is not supported by this app version.",
                false,
            ));
        }
        if payload.files.is_empty() || payload.files.len() > MAX_FILES {
            return Err(UserFacingError::new(
                "The saved CS2 profile has an invalid file count.",
                false,
            ));
        }
        for required in REQUIRED_CORE {
            if !payload
                .files
                .iter()
                .any(|file| file.scope == "userdata" && file.relative_path == required)
            {
                return Err(UserFacingError::new(
                    format!("The saved CS2 profile is missing {required}."),
                    false,
                ));
            }
        }
        Ok(())
    }

    fn discover_steam_context() -> Result<SteamContext, UserFacingError> {
        if !is_process_running("steam.exe") {
            return Err(UserFacingError::new(
                "Войдите в Steam, чтобы Game Passport смог определить ваши настройки CS2.",
                true,
            ));
        }
        let steam_root = find_steam_root()
            .ok_or_else(|| UserFacingError::new("Steam installation could not be found.", true))?;
        let account_id = active_account_id().filter(|id| *id != 0).ok_or_else(|| {
            UserFacingError::new(
                "Войдите в Steam, чтобы Game Passport смог определить ваши настройки CS2.",
                true,
            )
        })?;
        let userdata_cfg = steam_root
            .join("userdata")
            .join(account_id.to_string())
            .join(APP_ID)
            .join("local")
            .join("cfg");
        if !userdata_cfg.is_dir() {
            return Err(UserFacingError::new("CS2 settings were not found for the active Steam user. Launch CS2 once, close it, and retry.", true));
        }
        let install_cfg = find_install_cfg(&steam_root);
        Ok(SteamContext {
            userdata_cfg,
            install_cfg,
        })
    }

    fn ensure_game_closed() -> Result<(), UserFacingError> {
        if is_process_running("cs2.exe") {
            Err(UserFacingError::new(
                "Закройте Counter-Strike 2 перед сохранением или применением настроек, затем нажмите «Повторить проверку».",
                true,
            ))
        } else {
            Ok(())
        }
    }

    fn find_steam_root() -> Option<PathBuf> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey("Software\\Valve\\Steam") {
            if let Ok(value) = key.get_value::<String, _>("SteamPath") {
                let path = PathBuf::from(value.replace('/', "\\"));
                if path.is_dir() {
                    return Some(path);
                }
            }
        }
        if let Ok(program_files) = env::var("ProgramFiles(x86)") {
            let path = PathBuf::from(program_files).join("Steam");
            if path.is_dir() {
                return Some(path);
            }
        }
        None
    }

    fn active_account_id() -> Option<u32> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let steam = hkcu.open_subkey("Software\\Valve\\Steam").ok()?;
        steam
            .open_subkey("ActiveProcess")
            .ok()
            .and_then(|key| key.get_value("ActiveUser").ok())
            .or_else(|| steam.get_value("ActiveUser").ok())
    }

    fn find_install_cfg(steam_root: &Path) -> Option<PathBuf> {
        let mut libraries = vec![steam_root.to_path_buf()];
        let library_file = steam_root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(library_file) {
            for line in text.lines() {
                let tokens = quoted_tokens(line);
                if tokens.first().map(String::as_str) == Some("path") && tokens.len() >= 2 {
                    libraries.push(PathBuf::from(tokens[1].replace("\\\\", "\\")));
                }
            }
        }
        libraries.sort();
        libraries.dedup();
        for library in libraries {
            let manifest = library.join("steamapps").join("appmanifest_730.acf");
            let Ok(text) = fs::read_to_string(manifest) else {
                continue;
            };
            let install_dir = text.lines().find_map(|line| {
                let tokens = quoted_tokens(line);
                (tokens.first().map(String::as_str) == Some("installdir") && tokens.len() >= 2)
                    .then(|| tokens[1].clone())
            });
            if let Some(name) = install_dir {
                let path = library
                    .join("steamapps")
                    .join("common")
                    .join(name)
                    .join("game")
                    .join("csgo")
                    .join("cfg");
                if path.is_dir() {
                    return Some(path);
                }
            }
        }
        None
    }

    fn quoted_tokens(line: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        let mut escaped = false;
        for character in line.chars() {
            if escaped {
                current.push(character);
                escaped = false;
                continue;
            }
            if character == '\\' && quoted {
                current.push(character);
                escaped = true;
                continue;
            }
            if character == '"' {
                if quoted {
                    result.push(current.clone());
                    current.clear();
                }
                quoted = !quoted;
            } else if quoted {
                current.push(character);
            }
        }
        result
    }

    fn capture_userdata_cfg_files(
        root: &Path,
        files: &mut Vec<Cs2ConfigFile>,
        details: &mut Vec<String>,
    ) -> Result<(), UserFacingError> {
        let entries = fs::read_dir(root)
            .map_err(|error| io_error("Could not read the CS2 configuration folder.", error))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(OsStr::to_str) else {
                continue;
            };
            if path.is_file() && name.to_ascii_lowercase().ends_with(".cfg") {
                add_file(files, "userdata", name, &path, true, details)?;
            }
        }
        Ok(())
    }

    fn capture_autoexec_chain(
        root: &Path,
        files: &mut Vec<Cs2ConfigFile>,
        details: &mut Vec<String>,
    ) -> Result<(), UserFacingError> {
        if !root.join("autoexec.cfg").is_file() {
            return Ok(());
        }
        let mut queue = VecDeque::from([PathBuf::from("autoexec.cfg")]);
        let mut visited = HashSet::new();
        while let Some(relative) = queue.pop_front() {
            if visited.len() >= 16 {
                details.push("Stopped following custom cfg files at the safe limit of 16.".into());
                break;
            }
            if !visited.insert(relative.clone()) {
                continue;
            }
            let path = root.join(&relative);
            if !path.is_file() {
                details.push(format!(
                    "Referenced cfg was not found: {}",
                    relative.display()
                ));
                continue;
            }
            let bytes = fs::read(&path)
                .map_err(|error| io_error("Could not read a custom CS2 cfg file.", error))?;
            let (sanitized, removed) = sanitize_custom_cfg(&bytes)?;
            if removed > 0 {
                details.push(format!(
                    "Removed {removed} potentially sensitive line(s) from {}.",
                    relative.display()
                ));
            }
            for reference in referenced_cfgs(&sanitized) {
                if safe_relative_path(&reference).is_ok() {
                    queue.push_back(PathBuf::from(reference));
                }
            }
            add_bytes(
                files,
                "install",
                &relative.to_string_lossy().replace('\\', "/"),
                sanitized,
            )?;
        }
        Ok(())
    }

    fn add_file(
        files: &mut Vec<Cs2ConfigFile>,
        scope: &str,
        relative: &str,
        path: &Path,
        sanitize: bool,
        details: &mut Vec<String>,
    ) -> Result<(), UserFacingError> {
        if files
            .iter()
            .any(|file| file.scope == scope && file.relative_path == relative)
        {
            return Ok(());
        }
        let bytes = fs::read(path)
            .map_err(|error| io_error("Could not read a CS2 configuration file.", error))?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(UserFacingError::new(
                format!("{relative} exceeds the safe 512 KB per-file limit."),
                false,
            ));
        }
        if matches!(relative, "cs2_machine_convars.vcfg" | "cs2_video.txt") {
            let (bytes, removed) = sanitize_hardware_config(&bytes)?;
            if removed > 0 {
                details.push(format!(
                    "Excluded {removed} hardware-specific value(s) from {relative}."
                ));
            }
            add_bytes(files, scope, relative, bytes)
        } else if sanitize {
            let (bytes, removed) = sanitize_custom_cfg(&bytes)?;
            if removed > 0 {
                details.push(format!(
                    "Removed {removed} potentially sensitive line(s) from {relative}."
                ));
            }
            add_bytes(files, scope, relative, bytes)
        } else if contains_sensitive_data(&bytes) {
            Err(UserFacingError::new(
                format!("{relative} contains data that may be sensitive, so capture was stopped."),
                false,
            ))
        } else {
            add_bytes(files, scope, relative, bytes)
        }
    }

    fn add_bytes(
        files: &mut Vec<Cs2ConfigFile>,
        scope: &str,
        relative: &str,
        bytes: Vec<u8>,
    ) -> Result<(), UserFacingError> {
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(UserFacingError::new(
                format!("{relative} exceeds the safe 512 KB per-file limit."),
                false,
            ));
        }
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        files.push(Cs2ConfigFile {
            scope: scope.into(),
            relative_path: relative.into(),
            content_base64: BASE64.encode(&bytes),
            sha256,
            size: bytes.len() as u64,
        });
        Ok(())
    }

    fn sanitize_custom_cfg(bytes: &[u8]) -> Result<(Vec<u8>, usize), UserFacingError> {
        let text = std::str::from_utf8(bytes).map_err(|_| {
            UserFacingError::new(
                "A custom CS2 cfg file is not valid UTF-8 and was not captured.",
                false,
            )
        })?;
        let mut output = String::new();
        let mut removed = 0;
        for line in text.lines() {
            if sensitive_line(line) {
                removed += 1;
            } else {
                output.push_str(line);
                output.push('\n');
            }
        }
        Ok((output.into_bytes(), removed))
    }

    fn sanitize_hardware_config(bytes: &[u8]) -> Result<(Vec<u8>, usize), UserFacingError> {
        let text = std::str::from_utf8(bytes).map_err(|_| {
            UserFacingError::new(
                "A CS2 machine configuration file is not valid UTF-8.",
                false,
            )
        })?;
        let blocked = [
            "sound_device_override",
            "audio_device",
            "vendorid",
            "deviceid",
            "monitor_index",
            "refreshrate_numerator",
            "refreshrate_denominator",
        ];
        let mut output = String::new();
        let mut removed = 0;
        for line in text.lines() {
            let lower = line.to_ascii_lowercase();
            if blocked.iter().any(|marker| lower.contains(marker)) {
                removed += 1;
            } else {
                output.push_str(line);
                output.push('\n');
            }
        }
        Ok((output.into_bytes(), removed))
    }

    fn sensitive_line(line: &str) -> bool {
        let value = line.trim().to_ascii_lowercase();
        [
            "password",
            "token",
            "cookie",
            "authorization",
            "connect ",
            "connect\t",
            "setinfo",
        ]
        .iter()
        .any(|marker| value.contains(marker))
    }

    fn contains_sensitive_data(bytes: &[u8]) -> bool {
        std::str::from_utf8(bytes)
            .map(|text| text.lines().any(sensitive_line))
            .unwrap_or(true)
    }

    fn referenced_cfgs(bytes: &[u8]) -> Vec<String> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return vec![];
        };
        let mut result = Vec::new();
        for line in text.lines() {
            let line = line.split("//").next().unwrap_or("");
            for command in line.split(';') {
                let mut parts = command.split_whitespace();
                let keyword = parts.next().unwrap_or("").to_ascii_lowercase();
                if keyword != "exec" && keyword != "execifexists" {
                    continue;
                }
                let Some(raw) = parts.next() else { continue };
                let mut value = raw.trim_matches(['"', '\'']).replace('\\', "/");
                if !value.to_ascii_lowercase().ends_with(".cfg") {
                    value.push_str(".cfg");
                }
                result.push(value);
            }
        }
        result
    }

    fn safe_relative_path(value: &str) -> Result<PathBuf, UserFacingError> {
        if value.is_empty() || value.len() > 160 {
            return Err(UserFacingError::new(
                "The CS2 profile contains an invalid file name.",
                false,
            ));
        }
        let path = PathBuf::from(value.replace('/', "\\"));
        let components = path.components().collect::<Vec<_>>();
        if components.is_empty()
            || components.len() > 4
            || components
                .iter()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(UserFacingError::new(
                "The CS2 profile contains an unsafe relative path.",
                false,
            ));
        }
        Ok(path)
    }

    fn validate_file_name(scope: &str, relative: &Path) -> Result<(), UserFacingError> {
        let value = relative
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase();
        let core = REQUIRED_CORE
            .into_iter()
            .chain(OPTIONAL_CORE)
            .any(|name| value == name);
        let cfg = value.ends_with(".cfg");
        let allowed = match scope {
            "userdata" => core || (cfg && relative.components().count() == 1),
            "install" => cfg,
            _ => false,
        };
        if allowed {
            Ok(())
        } else {
            Err(UserFacingError::new(
                "The CS2 profile contains a file type that Game Passport will not write.",
                false,
            ))
        }
    }

    fn backup_root() -> Result<PathBuf, UserFacingError> {
        Ok(cs2_backup_directory()?.join(Utc::now().format("%Y%m%d-%H%M%S").to_string()))
    }

    fn cs2_backup_directory() -> Result<PathBuf, UserFacingError> {
        let local = env::var("LOCALAPPDATA").map_err(|_| {
            UserFacingError::new(
                "Windows Local AppData folder could not be determined.",
                false,
            )
        })?;
        Ok(PathBuf::from(local)
            .join("Game Passport")
            .join("Backups")
            .join("CS2"))
    }

    fn rollback(committed: &[(&DecodedFile, Option<PathBuf>)]) {
        for (item, previous) in committed.iter().rev() {
            let _ = fs::remove_file(&item.target);
            if let Some(path) = previous {
                let _ = fs::rename(path, &item.target);
            }
        }
    }

    fn cleanup_staged(staged: &[(&DecodedFile, PathBuf)]) {
        for (_, path) in staged {
            let _ = fs::remove_file(path);
        }
    }

    fn io_error(message: &str, error: std::io::Error) -> UserFacingError {
        UserFacingError::new(message, true).detail(error.to_string())
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
                    let length = entry
                        .szExeFile
                        .iter()
                        .position(|value| *value == 0)
                        .unwrap_or(entry.szExeFile.len());
                    if String::from_utf16_lossy(&entry.szExeFile[..length])
                        .eq_ignore_ascii_case(expected)
                    {
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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_steam_library_path() {
            assert_eq!(
                quoted_tokens(r#"\t\t"path"\t\t"D:\\SteamLibrary""#),
                vec!["path", r#"D:\\SteamLibrary"#]
            );
        }

        #[test]
        fn follows_only_safe_exec_references() {
            let values =
                referenced_cfgs(b"exec crosshair\nexec \"sub/binds.cfg\"\nsensitivity 1.2\n");
            assert_eq!(values, vec!["crosshair.cfg", "sub/binds.cfg"]);
            assert!(safe_relative_path(&values[1]).is_ok());
            assert!(safe_relative_path("../secret.cfg").is_err());
        }

        #[test]
        fn removes_credentials_and_fixed_refresh_rate() {
            let (custom, removed) =
                sanitize_custom_cfg(b"sensitivity 1.0\nrcon_password secret\n").unwrap();
            assert_eq!(removed, 1);
            assert_eq!(String::from_utf8(custom).unwrap(), "sensitivity 1.0\n");
            let (video, removed) = sanitize_hardware_config(
                b"\"setting.defaultres\" \"1920\"\n\"setting.refreshrate_numerator\" \"240\"\n",
            )
            .unwrap();
            assert_eq!(removed, 1);
            assert!(String::from_utf8(video).unwrap().contains("defaultres"));
        }
    }
}

#[cfg(target_os = "windows")]
pub fn capture() -> Cs2CommandResponse {
    windows::capture()
}

#[cfg(target_os = "windows")]
pub fn apply(payload: Cs2Payload) -> Cs2CommandResponse {
    windows::apply(payload)
}

#[cfg(target_os = "windows")]
pub fn preflight() -> Cs2CommandResponse {
    windows::preflight()
}

#[cfg(target_os = "windows")]
pub fn restore() -> Cs2CommandResponse {
    windows::restore()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_uses_camel_case_json_fields() {
        let payload = Cs2Payload {
            schema_version: 1,
            captured_at: "now".into(),
            files: vec![],
            total_bytes: 0,
            core_files_found: vec![],
            optional_files_missing: vec![],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["schemaVersion"], 1);
        assert!(json.get("totalBytes").is_some());
    }
}
