# Changelog

Изменения репозитория DDS 48–52 МГц: плата, прошивка, DDS Studio. Формат: [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/).
Версия проекта — [SemVer](https://semver.org/lang/ru/); у частей свои версии (см. [VERSION_MANAGEMENT.md](VERSION_MANAGEMENT.md)).
Подробные журналы частей: [studio/CHANGELOG.md](studio/CHANGELOG.md), [firmware/README.md](firmware/README.md), [docs/REVISION_F.md](docs/REVISION_F.md).

## [Unreleased]

## [1.0.0] - 2026-11-09

Первая версия проекта с единым учётом версий. Состав: плата **ревизия F**, прошивка **1.1.0**, DDS Studio **0.2.0**.

### Добавлено
- Единый источник версий `config/version.json` и `tools/update_version.py` (status, sync, bump, firmware, board, studio).
- DDS Studio 0.2.0: раздел «Ревизии платы» и разработчик в боковой панели.
- MCP-сервер Konnect для KiCad 10 описан в навыках `pcb-design-pipeline` и `pcb-revision-eco`.
- CI: ручной запуск собирает установщики macOS (universal) и Windows x64.
- Шпаргалка [ECO_STUDIO_ШПАРГАЛКА.md](ECO_STUDIO_ШПАРГАЛКА.md).
