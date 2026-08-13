# PUBG Adapter v0.5.0 — технический контракт и Windows smoke test

## Подтверждённая структура

PUBG использует Steam App ID `578080`, executable `TslGame.exe` и Unreal Engine user settings. Game Passport ищет Steam install только для Diagnostics через registry, `steamapps/libraryfolders.vdf` и `appmanifest_578080.acf`. Настройки пользователя берутся независимо от Steam-пути из `%LOCALAPPDATA%\TslGame\Saved\Config\WindowsNoEditor` с fallback `Windows`.

`GameUserSettings.ini` обязателен. `Input.ini` и `Scalability.ini` захватываются при наличии. `TslPersistantData` сохраняется как одно структурированное INI-значение: parser не делит вложенные скобки по запятым, поэтому custom input, sensitivities и gameplay/audio preferences не повреждаются.

Источники исследования:

- PUBG Support, актуальный Windows path/install/executable: https://support.pubg.com/hc/en-us/articles/900002196723-GENERAL-CRASHING-AND-PERFORMANCE-GUIDE
- Steam, App ID 578080: https://store.steampowered.com/app/578080/PLAYERUNKNOWNS_BATTLEGROUNDS/
- Unreal Engine `UGameUserSettings`: https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/UGameUserSettings
- Unreal Engine config/INI syntax: https://dev.epicgames.com/documentation/unreal-engine/configuration-files-in-unreal-engine
- Windows Tool Help process snapshots: https://learn.microsoft.com/windows/win32/toolhelp/snapshots-of-the-system

## Безопасность и cross-PC

Cloud snapshot содержит только allowlisted file IDs и entries. Hardware/auth markers отбрасываются до сериализации, SHA-256 проверяется перед Apply, число файлов/entries и размер values ограничены, traversal запрещён. Apply сохраняет неизвестные и аппаратные строки целевого ПК, удаляет только совпадающие portable keys и добавляет сохранённые values.

BattlEye-safe принцип: только закрытая игра, конфигурационные файлы, Windows API, публичный NVAPI и поддерживаемые mouse HID interfaces. Никаких hooks, injection, memory/process access, macros или anti-cheat bypass.

## Smoke test на реальном Windows-ПК

1. Запустить PUBG один раз, настроить заметную sensitivity, несколько bindings, graphics/audio и разрешение; полностью закрыть игру.
2. Создать профиль PUBG, нажать «Сохранить текущие настройки». Проверить строки Gameplay/Keybinds/Graphics/Audio; отсутствующая категория обязана быть Warning.
3. Открыть Diagnostics: PUBG detected, config directory, найденные файлы, process `Closed`, category counts и capture result.
4. Изменить в PUBG те же значения, закрыть игру, нажать Apply. Проверить Display с точным разрешением и максимальной доступной частотой, профиль NVIDIA `TslGame.exe`, Mouse readback или честный Warning.
5. Запустить PUBG и проверить значения. Закрыть игру и проверить `Restore pre-Game Passport`.
6. Повторить с профилем из Supabase на втором Windows-ПК. Абсолютный путь первого ПК не должен присутствовать в copied Diagnostics/profile JSON.
7. Негативный тест: оставить PUBG запущенной. Capture/Apply должны остановиться; после закрытия кнопка Retry должна продолжить без перезапуска Game Passport.

Field tests A–E также встроены в Diagnostics.
