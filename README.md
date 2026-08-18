# Game Passport v0.6.0 RC

Windows Release Candidate существующего Game Passport на Tauri 2, React, TypeScript, Rust и Supabase. Реальные адаптеры CS2, PUBG, Display, NVIDIA и Mouse сохранены; Dota 2 убрана из основного пользовательского пути.

## Как установить в компьютерном клубе

1. Возьмите файл `Game Passport_0.6.0_x64-setup.exe` из Windows CI artifact `game-passport-windows-*`.
2. Запустите EXE обычным пользователем Windows. Постоянные права администратора не нужны.
3. Выберите язык и завершите установку. Установщик создаёт ярлыки в меню «Пуск» и на рабочем столе; WebView2 включён внутрь инсталлятора и не требует отдельной загрузки.
4. Запустите Game Passport и войдите в аккаунт Supabase. Пароль Steam приложение не запрашивает.
5. Для CS2 войдите в Steam. Закройте CS2/PUBG перед сохранением или применением.
6. Пройдите `Initial setup`, затем `Test Game Passport`. После входа в Steam проверку можно повторить без перезапуска приложения.
7. В `Diagnostics` нажмите `Save report` и передайте сохранённый JSON при найденной проблеме.

Удаление: `Параметры Windows → Приложения → Game Passport → Удалить`. Деинсталлятор удаляет программу и ярлыки. Локальные данные удаляются только при выборе соответствующего пункта деинсталлятора.

## Что проверяет Release Candidate

- Initial Setup: версия Windows, сеть, Supabase, Steam и активный Steam-пользователь, CS2, PUBG config, мониторы, NVIDIA и мышь. Предупреждения не блокируют вход в приложение.
- Test Game Passport: отдельные статусы `PASS / FAIL / WARNING / SKIPPED` для программного Capture/Apply и ручного подтверждения игры, изображения, дисплея, NVIDIA и мыши.
- Diagnostics: версия `0.6.0 RC`, build id, реальная доступность облака, права, политика обновления, путь логов, состояние адаптеров и обезличенный отчёт.
- Production logging: JSONL в `%LOCALAPPDATA%\app.gamepassport.desktop\logs`, ротация 1 MB × 3 файла, без паролей, токенов, cookies и Steam auth.
- Offline: ранее синхронизированные профили доступны только из локального кэша при ещё действующей локальной сессии. Вход, создание, изменение и удаление профилей офлайн не имитируются.
- Demo mode всегда помечен как `DEMO MODE`, не запускает адаптеры и не заявляет успешную проверку оборудования.

## Сборка и артефакты

Windows workflow: `.github/workflows/windows-build.yml`. Он выполняет frontend tests/build, Rust format/tests, Windows MSVC + native NVAPI bridge check, затем обязан получить оба инсталлятора:

```text
Game Passport_0.6.0_x64-setup.exe   (NSIS, основной)
Game Passport_0.6.0_x64_en-US.msi   (WiX, дополнительный)
SHA256SUMS.txt
```

Фактические имена задаёт Tauri bundler и могут отличаться пробелами/дефисами. CI завершится ошибкой, если EXE или MSI отсутствует. Этот macOS-хост не может честно выполнить Windows installer smoke test; готовность инсталлятора подтверждается только успешным Windows job и установкой на чистом тестовом ПК.

Ручной запуск workflow по умолчанию создаёт `field-test` сборку. Она использует локальные профили, но запускает настоящие Windows-адаптеры и мастер проверки. Это позволяет начать тестирование в клубе до подключения Supabase; перенос профиля между двумя ПК в таком режиме не проверяется.

Для `production` сборки в приватном GitHub-репозитории должны быть добавлены два Actions secret: `VITE_SUPABASE_URL` и `VITE_SUPABASE_ANON_KEY`. Workflow намеренно останавливает только production-сборку, если они отсутствуют или содержат значения-заглушки.

Локальные проверки разработки:

```powershell
pnpm install --frozen-lockfile
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc
pnpm tauri build -- --target x86_64-pc-windows-msvc
```

## Ограничения RC

- Реальный PASS для игры/изображения/оборудования требует проверки пользователем на целевом Windows-ПК; API-success хранится отдельно.
- NVIDIA переносит разрешённый набор публичных NVAPI DRS параметров, scaling, выходной формат/диапазон/глубину цвета и системную brightness/contrast/gamma LUT. Неизвестные или несовместимые значения дают Warning; недокументированные private NVAPI вызовы не используются.
- DPI/polling меняются только для явно поддержанных физических устройств; виртуальные, неоднозначные и неизвестные мыши не получают Success.
- Аппаратные ID, monitor/audio device paths, Steam credentials/tokens/cookies и игровые anti-cheat файлы не переносятся.
- Автообновление не активировано: нет фиктивного endpoint и нет неподписанной установки. RC обновляется вручную только доверенным подписанным инсталлятором.

Технические детали: [Windows RC](docs/WINDOWS_RC.md), [CS2](docs/CS2_ADAPTER.md), [PUBG](docs/PUBG_ADAPTER.md), [Display/NVIDIA](docs/DISPLAY_NVIDIA_ADAPTERS.md), [Mouse](docs/MOUSE_PASSPORT.md).
