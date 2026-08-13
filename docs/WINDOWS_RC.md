# Windows RC validation and release policy

## Installer and permissions

The primary package is NSIS `-setup.exe`; WiX MSI is built as a secondary enterprise option. NSIS uses `currentUser`, installs under `%LOCALAPPDATA%\Game Passport`, requests no permanent elevation, creates Start/Desktop shortcuts, registers uninstall, and embeds the offline WebView2 installer. Production WebView devtools are disabled.

The Tauri capability contains only `core:default`. No shell, generic filesystem, process, dialog or updater plugin permission is granted. Game settings are accessed only by the existing narrowly-scoped Rust commands. The app does not inject DLLs, read game memory, control anti-cheat, or request Steam credentials.

## Field matrix

Run on at least one supported Windows 10 22H2 PC and one Windows 11 PC. For each CS2 and PUBG profile:

1. Capture with the game closed and a distinctive known configuration.
2. Change settings in game, close the game, then Apply.
3. Launch the game and record Gameplay and Visual confirmations.
4. Confirm Windows display mode.
5. Confirm NVIDIA DRS only when the adapter reports support.
6. Confirm mouse DPI/polling only when the physical adapter reports read/write and re-read verification.
7. Repeat Apply on a second PC and export Diagnostics.

Software Capture/Apply results and user confirmation are intentionally separate. Any adapter error is FAIL; unsupported/partial/fallback is WARNING. The UI never promotes either to PASS.

## Offline behavior

Successful Supabase profile lists are cached locally per user. A still-valid cached Supabase session may open those cached profiles offline. Capture can read locally, but syncing the captured profile must fail visibly while offline. Apply can use an already cached profile. Authentication and all profile mutations remain server-authoritative; no offline success queue exists.

## Logs and reports

Production log directory:

```text
%LOCALAPPDATA%\app.gamepassport.desktop\logs
```

`game-passport.log` is JSON Lines with timestamp, severity, adapter, operation, error code, friendly message and bounded technical details. At 1 MB it rotates to `.1`, `.2`, `.3`. Known credentials/tokens/cookies and JWT-looking values are redacted; `%USERPROFILE%` replaces the Windows home path.

Saved reports:

```text
%USERPROFILE%\Documents\Game Passport Reports
```

Reports exclude email, user id, auth fields, profile settings/payloads, secrets and raw home directory. Copy and Save use the same sanitized report body.

## Update foundation

The build exposes version + build id and identifies the channel as `Manual signed releases (updater not activated)`. CI produces checksums. A future updater may be enabled only after all of these exist:

- controlled HTTPS release endpoint;
- offline-protected private signing key and embedded public verification key;
- signed update artifacts and signed manifest;
- staged rollback and compatibility policy;
- field-tested NSIS upgrade/uninstall behavior.

No updater plugin, endpoint, fake manifest or unsigned automatic execution is enabled in this RC.

## Release gate

CI must pass 31 frontend tests and 43 Rust tests or their later supersets, production frontend build, Rust formatting, Windows MSVC compilation including `native/nvapi_bridge.cpp`, and generation of both NSIS EXE and WiX MSI. The staged artifact includes `SHA256SUMS.txt`. Final installer PASS requires a clean-PC install, launch, shortcut, Initial Setup, field workflow and uninstall smoke test on Windows.
