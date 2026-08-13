; Game Passport is installed per-user and does not request elevation.
; The stock Tauri template already creates the Start menu shortcut and removes
; both shortcuts during uninstall. This hook makes the desktop shortcut default.
!macro NSIS_HOOK_POSTINSTALL
  Call CreateOrUpdateDesktopShortcut
!macroend
