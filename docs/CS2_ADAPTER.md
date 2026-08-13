# CS2 Adapter: техническое исследование и контракт v1

Дата проверки: 11 августа 2026 года. App ID Counter-Strike 2 — `730`.

## Подтверждённая структура

Steam хранит данные игр по схеме `Steam\userdata\<account-id>\<app-id>`. Это подтверждено официальной справкой Steam Cloud: <https://help.steampowered.com/en/faqs/view/68D2-35AB-09A9-7678>.

Для CS2 текущий локальный каталог настроек:

```text
<Steam>\userdata\<active-account-id>\730\local\cfg\
```

Основные файлы:

- `cs2_user_convars_0_slot0.vcfg` — sensitivity, zoom sensitivity, crosshair, viewmodel, radar, HUD, gameplay, audio/voice convars и другие сохраняемые console variables;
- `cs2_user_keys_0_slot0.vcfg` — key binds;
- `cs2_machine_convars.vcfg` — настройки, часть которых относится к конкретному компьютеру;
- `cs2_video.txt` — разрешение, оконный режим и расширенные графические настройки.

Названия `cs2_user_convars_0_slot0.vcfg` и `cs2_user_keys_0_slot0.vcfg` также видны в журнале самой CS2, приложенном к ValveSoftware issue: <https://github.com/ValveSoftware/csgo-osx-linux/issues/3239>. Актуальный практический перечень всех четырёх файлов дополнительно сверялся по обсуждению Steam от мая 2025 года: <https://steamcommunity.com/app/730/discussions/0/7342617747181253583/>.

Пользовательский `autoexec.cfg` обычно находится в:

```text
<Steam Library>\steamapps\common\<CS2 installdir>\game\csgo\cfg\autoexec.cfg
```

Адаптер не предполагает стандартный диск `C:`. Он читает SteamPath из Windows Registry, затем `steamapps\libraryfolders.vdf` и `appmanifest_730.acf`, чтобы найти фактическую библиотеку и `installdir`.

## Определение пользователя без чтения авторизации

Используется только числовой `ActiveUser` из `HKCU\Software\Valve\Steam\ActiveProcess` (с безопасным fallback на корневое значение Steam). Он нужен исключительно для выбора локальной папки `userdata`.

Не читаются:

- `loginusers.vdf`;
- `localconfig.vdf`, `sharedconfig.vdf`;
- Steam cookies, SSFN, refresh/access tokens;
- логин, пароль или Steam Guard;
- `remotecache.vdf` и cloud metadata.

SteamID/AccountID и абсолютные пути не записываются в профиль и не отправляются в Supabase.

## Что сохраняется

В JSONB `profiles.settings.adapters["game.cs2"]` сохраняется payload schema version 1:

- относительное безопасное имя файла;
- scope `userdata` или `install`;
- Base64-содержимое;
- размер и SHA-256;
- дата capture, список найденных core-файлов и отсутствующих optional-файлов.

Кроме четырёх core-файлов сохраняются:

- пользовательские `.cfg` верхнего уровня из `730\local\cfg`;
- `autoexec.cfg`;
- пользовательские cfg, на которые autoexec ссылается через `exec`/`execifexists`, рекурсивно до безопасного лимита.

Системные cfg, поставляемые с игрой, целиком не копируются.

## Безопасность capture/apply

- операция блокируется, если запущен `cs2.exe`;
- требуется запущенный и авторизованный Steam;
- только относительные пути без `..`, drive prefix и UNC;
- разрешены только известные core-файлы и `.cfg`;
- максимум 32 файла, 512 КБ на файл и 2 МБ суммарно;
- перед apply повторно проверяются Base64, размер и SHA-256;
- строки custom cfg с `password`, `token`, `cookie`, `authorization`, `connect` или `setinfo` не попадают в облако;
- перед заменой создаётся backup в `%LOCALAPPDATA%\Game Passport\Backups\CS2\<timestamp>`;
- файлы сначала записываются во временные цели; при ошибке выполняется rollback уже заменённых файлов;
- непосредственно перед commit ещё раз проверяется, что CS2 не запущена.

## Намеренно не переносится

Официальная документация Steamworks рекомендует не синхронизировать машинно-зависимые настройки: <https://partner.steamgames.com/doc/features/cloud>. Поэтому из machine/video-файлов удаляются:

    - фиксированная refresh rate — Display Adapter v0.3.0 выбирает максимально поддерживаемое значение для сохранённого разрешения;
- GPU VendorID/DeviceID;
- monitor index;
- конкретный audio device override/GUID.

Также не переносятся:

- Steam launch options: они находятся в клиентских account-конфигах, которые Game Passport принципиально не читает;
- Steam Input/controller layout;
- workshop content, карты, demos, screenshots;
- cloud metadata и файлы `*_lastclouded`;
- произвольные cfg из каталога установки, если они не достижимы из autoexec;
- runtime-only convars, которые сама CS2 не записала в поддерживаемые файлы.

Общая громкость, voice-настройки и portable audio convars переносятся; привязка к конкретному физическому устройству — нет.

## Ограничение проверки результата

Game Passport может достоверно подтвердить, что файлы прошли проверку и были записаны с backup/rollback. Но окончательно подтвердить, что CS2 приняла каждое значение, можно только после запуска игры: драйвер, монитор или новая версия CS2 могут скорректировать unsupported video values. Поэтому применение machine/video-файлов возвращает `Warning`, а не безусловный `Success`.

Если Valve изменит имена или формат обязательных файлов, capture завершится Error и ничего не сохранит. Это сделано намеренно, чтобы не выдавать ложный Success.
