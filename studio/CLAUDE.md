# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

DDS Studio: a desktop (Tauri 2, macOS + Windows) and web app for controlling and studying the АО НПЦ «ЭЛВИС» **1508ПЛ8Т** (двухканальный DDS 1 ГГц с 48-битным аккумулятором) and its **DDS 48–52 МГц, ревизия F** evaluation board. All engineering logic is Rust (`crates/dds-core`); the UI is React 19 + Tailwind 4 (`src/`). UI language: Russian; register/field names stay in English as in the datasheet.

## Commands

```bash
npm ci                         # first time
npm run app                    # desktop app, hot reload (Tauri dev; Vite on :1420)
make dev                       # web dev: dds-server :7878 + Vite :1420 (Vite proxies /api)
npm run web                    # build UI, run release dds-server on http://127.0.0.1:7878

cargo test -p dds-core         # model tests (registers, bus framing, model, simulator, RPC)
npm run build                  # tsc --noEmit + vite build
make check                     # version status + build + tests + cargo check
cargo run -p dds-core --example probe   # CLI sanity probe of the model

python3 build_macos.py [--universal] [--with-server]   # .app + .dmg → release/vX.Y.Z/
.\build_windows.ps1 [-Bundles nsis|msi] [-WithServer]  # NSIS/MSI → release\vX.Y.Z\
```

## Versioning and releases

- Single source of truth: `config/version.json` (`app_info.version`, `release_date`, `last_updated`). Never edit versions by hand. Use `python3 tools/update_version.py status | bump patch|minor|major | sync [X.Y.Z]`. It updates `Cargo.toml` (`[workspace.package]`), `Cargo.lock`, `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, the README heading and `.github/ISSUE_TEMPLATE/bug.yml`.
- `CHANGELOG.md` is hand-written (Keep a Changelog): `## [X.Y.Z] - YYYY-MM-DD`.
- Commit subjects: user-facing description, then `; bump to X.Y.Z` when the version changes.
- Build scripts refuse to build if `status` reports drift. GitHub runs workflows only from the repo root `DDS/.github/workflows/`; `studio/.github/` is inert. Installers (macOS universal .dmg, Windows .exe/.msi) are built by the `desktop` job of the root `ci.yml` on manual `workflow_dispatch`. Releases: the root `.github/workflows/release.yml` builds both installers into a draft GitHub Release on tag `studio-vX.Y.Z` (or manual run with the tag).

## Architecture

**One RPC for both runtimes.** `dds_core::Engine::handle(cmd, args) -> Result<Value>` (`crates/dds-core/src/api.rs`) is the whole backend API. Tauri exposes it as the `rpc` command (`src-tauri/src/lib.rs`), `dds-server` as `POST /api/rpc` → `{ok, data|error}`. The frontend calls `rpc()` from `src/lib/api.ts`, which picks the transport. Add features by adding an RPC command in `api.rs` and mirroring its types in `src/lib/types.ts`.

**dds-core modules**
- `regs.rs` — `REGS` (16-bit address, name, reset, group) and `FIELDS` (bit positions from the datasheet, table 5.1; access, RU title/description, options, block, unit, signed). Profiles: `N_PROF1` = 16 for channel 1, `N_PROF2` = 4 for channel 2 (the chip has 64 each, at base + 0x400 + 0x10·y). `defaults()` vs `starter()` (working board config, 50 MHz at 480 MHz clock), `write_order()` (CTR last), `COMMIT_REG = None`.
- `bus.rs` — 24-bit SPI frames: NOP 0x00, SETA 0x10, WR 0x20, WRI 0x30, SETAFT 0xB0; decoding for the log.
- `model.rs` — `Settings` (Fclk, RREF, filter Q, band, step, target level, sim toggles), Fout = dPh·Fclk/2⁴⁸, frequency plan + DIP code, bit-exact channel model and spectrum, ABCD output-filter model, phase noise, LFM and keying plans, `derive()` → `{metrics, issues}`.
- `sim.rs` — `VirtualChip` + virtual board firmware: SPI shift register, SWRST/CLR/LINK, LFM stages, profile hopping, PGOOD, clock presence, DIP.
- `transport.rs` — `Link` trait: sim or `FirmwareLink` (USB-UART 38400 to the board MCU D2, firmware ≥ 1.1.0, protocol in `docs/PROTOCOL_ru.md`); port listing, USB scan, `PINS`.
- `api.rs` — `Engine`: settings + regs persistence (JSON), auto-apply writes changed registers in `write_order()`, bus transaction log (`Txn` with decoded field changes), snapshot.

**Frontend** — `src/lib/store.ts` (zustand): `snap` (full snapshot after every mutating RPC), `status` (polled every 350 ms), log, view routing. `src/components/fields.tsx` renders any register field from metadata. Views live in `src/views/*View.tsx`; the overview diagram is drawn from `src/lib/blocks.ts`, whose ids match `FieldDef.block`. Board facts live in `src/lib/board.ts`, lessons in `src/lib/lessons.ts`. Charts are the in-house `components/Plot.tsx` (SVG, log axes, dual y, hover).

## Conventions

- UI strings and model messages are Russian. Datasheet names stay English.
- Register bit positions come only from the datasheet — never guess; `regs.rs` tests guard overlaps, defaults and field round-trips.
- Keep physics in Rust; the frontend formats and plots. Do not duplicate model math in TS beyond trivial display formulas.
- `tsconfig` is strict with `noUnusedLocals/Parameters`; `npm run build` must pass.
- lucide-react 1.x has no brand icons (e.g. no `Github`).
- Board firmware lives in `../firmware` (D2, 1887ВЕ7Т): UART commands in `src/app/remote.c`, parsing in `src/core/cmdline.c`, host tests in `tests/`. Keep the protocol in `transport.rs`, `docs/PROTOCOL_ru.md` and `../firmware/README.md` §8.5 in sync.
- Hardware safety notes must stay in docs/UI: D2 runs at 5 V (5 V USB-UART adapter, adapter VCC not connected, disconnect it for ISP on X1), clock on XW1 −10…+5 dBm, 50 Ω load on XW2.
