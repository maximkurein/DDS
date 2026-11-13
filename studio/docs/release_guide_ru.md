# Выпуск релиза DDS Studio

Связанное: [VERSION_MANAGEMENT.md](../VERSION_MANAGEMENT.md) · [BUILD.md](../BUILD.md) · [CHANGELOG.md](../CHANGELOG.md)

## Чек-лист

1. **Версия**
   ```bash
   python3 tools/update_version.py bump patch     # или minor / major
   ```
2. **CHANGELOG.md** — `## [Unreleased]` → `## [X.Y.Z] - YYYY-MM-DD`, добавить новый пустой `## [Unreleased]` сверху.
3. **Проверка**
   ```bash
   python3 tools/update_version.py status         # все строки OK
   make check
   ```
4. **Ручная проверка** — `npm run app` (настольное приложение на egui) на виртуальной плате: Обзор, Частота (установить 50 МГц → профиль 1 и `SEL_REG` в журнале), Регистры (прочитать группу → readback ✓), Спектр, Тракт и шум, Свип, Перебор, Модуляция (ЛЧМ), Подключение (DEVID 0x0201). С платой — несущая на анализаторе и `REMOTE 0` в конце. Прошивка: тесты `../firmware` (`ctest --preset host-tests`).
5. **Коммит**
   ```bash
   git commit -am "Краткое описание для пользователя; bump to X.Y.Z"
   git push
   ```
6. **Сборка**
   - локально: `python3 build_macos.py --universal --with-server` (Mac) и `.\build_windows.ps1 -WithServer` (Windows) → `release/vX.Y.Z/` + `checksum.txt`;
   - или в GitHub: `git tag -a studio-vX.Y.Z -m "..." && git push origin studio-vX.Y.Z` → workflow **Release** соберёт `.dmg` (universal) и `.zip` для Windows и создаст черновик релиза.
7. **Публикация** — проверьте черновик релиза, вставьте раздел из CHANGELOG и контрольные суммы, опубликуйте.

## Именование артефактов

| Платформа | Файл |
|---|---|
| macOS | `DDS-Studio-vX.Y.Z-macos-<arm64\|x64\|universal>.dmg`, `….app.zip` |
| Windows | `DDS-Studio-vX.Y.Z-windows-x64-setup.exe`, `DDS-Studio-vX.Y.Z-windows-x64.zip` (`DDS Studio.exe`) |
| Сервер | `dds-server-vX.Y.Z-<платформа>` |
| Контрольные суммы | `checksum.txt` (SHA-256) |
