# Display + NVIDIA Adapters: технический контракт v1

Дата исследования: 11 августа 2026 года. Версия приложения: Game Passport v0.3.0.

## Официальные API

Display Adapter использует только документированные Microsoft API:

- `EnumDisplayDevicesW`: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaydevicesw>
- `EnumDisplaySettingsExW`: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsexw>
- `ChangeDisplaySettingsExW`: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-changedisplaysettingsexw>

NVIDIA Adapter использует официальный публичный NVAPI SDK Release 590:

- официальный MIT-репозиторий и static libraries: <https://github.com/NVIDIA/nvapi>
- DRS API: <https://docs.nvidia.com/nvapi/group__drsapi.html>
- Display Control и `NV_SCALING`: <https://docs.nvidia.com/nvapi/group__dispcontrol.html>

SDK vendored в `src-tauri/vendor/nvapi`; лицензия сохранена в `License.txt`. Фактическую реализацию NVAPI предоставляет установленный NVIDIA Driver.

## Display snapshot

В Supabase сохраняется только нормализованная политика:

```json
{
  "schemaVersion": 1,
  "width": 1280,
  "height": 960,
  "aspectRatio": "4:3",
  "displayMode": "fullscreen",
  "scalingPreference": "driver_managed",
  "refreshRatePolicy": "MAX_AVAILABLE"
}
```

Имя/модель монитора, Windows device name и конкретные Hz не сохраняются. При Apply используется GDI primary display. В multi-monitor конфигурации остальные display не изменяются.

Алгоритм выбора:

1. оставить только точное `width × height`;
2. оставить modes не ниже 32 bpp;
3. если есть progressive modes — исключить interlaced;
4. выбрать максимальный `dmDisplayFrequency`;
5. при одинаковой частоте предпочесть mode с большей color depth;
6. выполнить `CDS_TEST`;
7. только затем применить с `CDS_UPDATEREGISTRY`.

Если точного разрешения нет, не применяется никакой fallback.

## NVIDIA snapshot

Профиль хранит `cs2.exe`, capture timestamp, массив `{id,key,value}` и документированный scaling enum. Не копируются NVIDIA driver database, binary export или hardware-specific profile blobs.

Разрешены только публичные DWORD IDs из `NvApiDriverSettings.h`:

| Key | Public ID |
|---|---:|
| power_management_mode | `0x1057EB71` |
| max_frame_rate | `0x10835002` |
| vertical_sync | `0x00A879CF` |
| texture_filtering_quality | `0x00CE2691` |
| shader_cache | `0x00198FFF` |
| shader_cache_size | `0x00AC8497` |
| anisotropic_filtering_mode | `0x10D2BB16` |
| anisotropic_filtering_level | `0x101E61A9` |
| anisotropic_sample_optimization | `0x00E73211` |
| anisotropic_filter_optimization | `0x0084CD70` |
| trilinear_optimization | `0x002ECAF2` |
| negative_lod_bias | `0x0019BB68` |
| maximum_pre_rendered_frames | `0x007BA09E` |
| fxaa | `0x1074C972` |
| mfaa | `0x0098C1AC` |
| preferred_refresh_rate | `0x0064B541` |

Capture сохраняет только явные current-profile overrides. Inherited global/base settings не превращаются в новые application overrides. При Apply неизвестные или отсутствующие в новой версии драйвера параметры пропускаются.

`preferred_refresh_rate` независимо от сохранённого значения применяется как публичное `REFRESH_RATE_OVERRIDE_HIGHEST_AVAILABLE`; фиксированная частота не переносится.

## NVIDIA application profile

DRS session загружает текущую driver database и ищет `cs2.exe` через `NvAPI_DRS_FindApplicationByName`. Если application отсутствует, создаётся профиль `Game Passport - Counter-Strike 2`, добавляется `cs2.exe` и вызывается `NvAPI_DRS_SaveSettings`.

Перед Apply считываются present/absent значения всего whitelist. Это локальный rollback snapshot. Restore возвращает прежние values и удаляет overrides, которых до Apply не существовало. Если Game Passport создал отдельный профиль поверх отсутствовавшего profile, best-effort restore удаляет именно профиль с именем Game Passport.

## 4:3 stretched

Для NVIDIA primary display считывается полная display config через `NvAPI_DISP_GetDisplayConfig`. Capture нормализует:

- `1` или `2` → `stretched`;
- `3` или `7` → `centered`;
- `5` или `6` → `aspect_ratio`;
- `8` → `integer_aspect`;
- `0` → `driver_default`.

Stretched соответствует документированному `NV_SCALING_GPU_SCALING_TO_NATIVE` / Force GPU – Full Screen. Apply изменяет scaling в свежей display config и вызывает `NvAPI_DISP_SetDisplayConfig` с `SAVE_TO_PERSISTENCE | DRIVER_RELOAD_ALLOWED`.

Если primary display не NVIDIA, details отсутствуют или API возвращает ошибку, scaling становится Warning/Unsupported. Registry fallback отсутствует.

## Backup и rollback

Display backup содержит только локальный Windows device name и прежний mode. Он не синхронизируется. При ошибке mode change прежний mode применяется немедленно.

NVIDIA backup содержит полный present/absent whitelist и scaling. Ошибка сохранения DRS запускает немедленный restore. Критическая ошибка следующего pipeline stage запускает adapter rollback через TypeScript orchestrator.

Rollback нельзя гарантировать при физическом отключении монитора, удалении драйвера, изменении topology или driver crash. В таких случаях UI показывает Error.

## Что требует реального Windows hardware

- фактический список EDID/driver modes конкретного monitor + cable + GPU;
- принятие 360/400/500 Hz конкретным соединением;
- визуальное подтверждение 4:3 stretched после запуска CS2;
- существование и доступность каждого whitelist ID в конкретной версии драйвера;
- поведение multi-GPU, Optimus/MUX, clone/spanning и remote desktop topology;
- создание NSIS/MSI и runtime loading `nvapi64.dll` на целевой машине.
