# Windows-сборка Game Passport v0.6.0 RC

Проект готов к сборке установщиков на GitHub Actions с MacBook. Компьютер клуба для этого не нужен.

Workflow `.github/workflows/windows-build.yml` запускается на настоящей Windows-машине GitHub и выдаёт один архив с тремя файлами:

- основной NSIS-установщик `.exe`;
- дополнительный MSI-установщик `.msi`;
- `SHA256SUMS.txt` для проверки целостности.

Ручной запуск по умолчанию выпускает Field Test-приложение: профили хранятся локально, а реальные Windows-адаптеры включены. Для production-сборки должны существовать секреты `VITE_SUPABASE_URL` и `VITE_SUPABASE_ANON_KEY`; их значения в лог не выводятся.

Локально на macOS подтверждены:

- 31 frontend-тест;
- TypeScript и production frontend build;
- 43 Rust-теста;
- Rust formatting;
- сборка Tauri-приложения без bundle.

Окончательно на Windows остаётся проверить установку, ярлыки, запуск, работу адаптеров с реальным оборудованием и удаление приложения. Это делается уже после получения `.exe`/`.msi`.
