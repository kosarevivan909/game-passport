# Mouse Passport v0.4.0 — технический контракт

Дата исследования: 11 августа 2026 года.

## Облачная модель

`settings.adapters.mouse` содержит только `schemaVersion`, `capturedAt`, `dpi` и optional `pollingRateHz`. Модель/бренд/VID/PID/path/serial/instance не сериализуются в payload и не попадают в Supabase. Благодаря этому профиль 800 DPI переносим между брендами.

## Общий контракт

Rust backend разделяет detection workflow и единый `MouseHardwareAdapter`: `readCurrentSettings`, `applyDpi`, `applyPollingRate`, `verifySettings`. Capture/apply/backup/restore вызывают его через выбранный transport. Новый бренд добавляется новой реализацией транспорта и capability probe без изменения profile schema.

Success означает реальный ответ устройства и совпавший readback. Сам факт `manufacturer == Logitech/Razer/Lamzu`, наличие фирменного ПО или успешный HID open недостаточны.

## Capability matrix

| Семейство | Detection | DPI read/write | Polling read/write | Статус аппаратной проверки |
|---|---:|---:|---:|---|
| Logitech VID 046D + HID++ `0x2201` | Да, runtime probe | Да; capabilities/read/set/readback | Только при runtime `0x8060`, 125–1000 Hz | Компилируется; конкретные модели требуют Test A–D на Windows |
| Razer Viper V2 Pro A5/A6 | Да, PID + protocol probe | 100–30000, step 50, read/set/readback | 125/500/1000 | Требует реального Windows field test |
| Razer DeathAdder V3 B2 | Да | 100–30000, step 50, read/set/readback | 125–8000 via polling2 | Требует field test |
| Razer DeathAdder V3 Pro B6/B7 | Да | 100–30000, step 50, read/set/readback | 125–4000 via polling2 | Требует field test |
| Razer Viper V3 Pro C0/C1 | Да | 100–35000, step 50, read/set/readback | 125–8000 via polling2 | Требует field test |
| Другой Razer PID | Да | Unsupported | Unsupported | Не заявляется |
| Lamzu VID 373E, включая Maya product string | Да | Unsupported | Unsupported | Публичный transport не найден |
| Другой HID mouse | Да, если HID usage Mouse | Unsupported | Unsupported | Не заявляется |

Таблица описывает реализованный protocol code, а не утверждает, что перечисленное железо физически тестировалось в этой macOS-среде. Runtime всё равно обязан успешно выполнить probe и readback; иначе результат Warning/Error.

## Logitech

Используется HID++ 2.0. Root feature discovery ищет Adjustable DPI `0x2201` и Adjustable Report Rate `0x8060`. DPI capabilities читаются с устройства, поэтому стандартные 400/800/1600 сохраняются точно при их наличии, а нестандартное значение округляется по реальному списку/шагу. Polling `0x8060` представляет интервалы 1/2/4/8 ms, то есть 1000/500/250/125 Hz.

Источники: [официальная документация Logitech HID++ 2.0](https://github.com/Logitech/cpg-docs/tree/master/hidpp20), [официальный список HID++ features](https://github.com/Logitech/cpg-docs/blob/master/hidpp20/README.md), [Microsoft HID API](https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/hid-api).

Ограничение: конкретный Logitech PID не получает capability по бренду; он должен реально expose нужный feature. Высокочастотный новый feature для 2000/4000/8000 Hz не реализован без публично подтверждённого протокола.

## Razer

Официальный Razer Chroma SDK относится к подсветке и не используется как DPI API. Backend общается только с PID из whitelist через HID feature report: 90-byte request, XOR CRC, status/class/command validation, затем повторное чтение DPI/polling. Старый polling command покрывает 125/500/1000; polling2 — 125–8000 в зависимости от model table.

Источники: [официальный Razer Chroma SDK](https://developer.razer.com/works-with-chroma/download/), [Microsoft HidD_SetFeature](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/hidsdi/nf-hidsdi-hidd_setfeature), [hidapi Windows-native backend](https://docs.rs/hidapi/latest/hidapi/).

Protocol constants независимо реализованы по наблюдаемому HID protocol. Код OpenRazer не vendored и не копируется в proprietary проект; проект использовался только как публичный исследовательский reference: [OpenRazer](https://github.com/openrazer/openrazer).

## Lamzu

Официальный download page предоставляет Aurora/Web Driver, но публичной спецификации команд DPI/polling нет. Копирование proprietary WebHID JavaScript запрещено условиями задачи и не выполнялось. Поэтому backend перечисляет Lamzu HID (VID 373E), модель из product descriptor и connection info, но capabilities read/write остаются Unsupported.

Источник: [официальная страница Lamzu Downloads / Aurora Web Driver](https://lamzu.com/pages/download).

## Выбор физической мыши

- перечисляются HID endpoints, а не установленные приложения;
- composite collections группируются по локальному physical key;
- serial, если доступен, участвует только в SHA-256 fingerprint в памяти/локальном backup и не показывается/не синхронизируется;
- controllable означает успешный brand protocol probe;
- один controllable physical fingerprint выбирается автоматически;
- два разных controllable fingerprints дают `selectionAmbiguous = true`; запись не выполняется;
- generic/virtual pointing device не выбирается только по HID usage.

## Normalize и partial success

`normalizeDesiredDpi` выбирает nearest supported и при равной дистанции — меньшее значение. Значение вне диапазона clamps к min/max. `normalizePollingRate` выбирает exact или наибольшее поддерживаемое `<= requested`; если requested ниже минимума — минимум. Любое отличие requested/applied отображается как Warning.

## Backup / Restore

Backup создаётся только после успешного чтения current DPI; polling сохраняется, если читается. Файл локальный. Restore требует тот же physical fingerprint, пишет snapshot и проверяет readback. Повреждённый schema/range/token отклоняется.

## Не реализовано

Lamzu write, Logitech >1000 Hz, неподтверждённые Razer PID, macros, buttons, RGB, debounce, angle snapping, lift-off distance, input generation и любые kernel/anti-cheat-sensitive механизмы.
