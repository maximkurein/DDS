#!/usr/bin/env python3
"""Версии репозитория DDS: проект, ревизия платы, прошивка, DDS Studio.

Единственный источник — config/version.json. Скрипт сверяет и расставляет версии
по файлам, где они должны совпадать. Только стандартная библиотека Python.

    python tools/update_version.py status
    python tools/update_version.py sync                       # из config/version.json во все файлы
    python tools/update_version.py bump patch|minor|major     # версия проекта
    python tools/update_version.py firmware bump patch|minor|major
    python tools/update_version.py firmware set X.Y.Z
    python tools/update_version.py board next                 # F -> G: только в начале новой ревизии (pcb-revision-eco)
    python tools/update_version.py board set G
    python tools/update_version.py studio <аргументы>         # передаётся в studio/tools/update_version.py
Флаг --dry-run показывает изменения без записи.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "config" / "version.json"
README = ROOT / "README.md"
FW_CMAKE = ROOT / "firmware" / "CMakeLists.txt"
MAKE_OUTPUTS = ROOT / "tools" / "make_outputs.sh"
STUDIO_TOOL = ROOT / "studio" / "tools" / "update_version.py"

SEMVER = r"\d+\.\d+\.\d+"


def rel(p: Path) -> str:
    return p.relative_to(ROOT).as_posix()


def load() -> dict:
    return json.loads(SOURCE.read_text(encoding="utf-8"))


def save(data: dict, dry: bool) -> None:
    if dry:
        print(f"[DRY RUN] {rel(SOURCE)}")
        return
    SOURCE.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Updated {rel(SOURCE)}")


def semver(v: str) -> tuple[int, int, int]:
    if not re.fullmatch(SEMVER, v):
        raise SystemExit(f"не semver: {v!r}")
    a, b, c = (int(x) for x in v.split("."))
    return a, b, c


def bumped(v: str, part: str) -> str:
    a, b, c = semver(v)
    return {"major": f"{a + 1}.0.0", "minor": f"{a}.{b + 1}.0", "patch": f"{a}.{b}.{c + 1}"}[part]


def board_files() -> list[Path]:
    return sorted(ROOT.glob("*.kicad_sch")) + sorted(ROOT.glob("*.kicad_pcb"))


# Каждый трекер: (метка, файл, регулярное выражение с одной группой, ключ в источнике).
# Все совпадения в файле должны равняться значению из источника.
def trackers() -> list[tuple[str, Path, str, str]]:
    t: list[tuple[str, Path, str, str]] = [
        ("project", README, r"badge/версия-(" + SEMVER + r")-", "project"),
        ("board", README, r"badge/ревизия-([A-Z])-", "board"),
        ("board", MAKE_OUTPUTS, r"_rev([A-Z])_", "board"),
        ("firmware", FW_CMAKE, r"project\(dds_fw\s+VERSION\s+(" + SEMVER + r")", "firmware"),
        ("firmware", README, r"badge/прошивка-(" + SEMVER + r")-", "firmware"),
        ("project", README, r'alt="Версия (' + SEMVER + r')"', "project"),
        ("board", README, r'alt="Ревизия ([A-Z])"', "board"),
        ("firmware", README, r'alt="Прошивка (' + SEMVER + r')"', "firmware"),
    ]
    for f in board_files():
        t.append(("board", f, r'\(title_block[^()]*(?:\([^()]*\)[^()]*)*?\(rev "([A-Z])"\)', "board"))
    return t


def expected(data: dict) -> dict[str, str]:
    c = data["components"]
    return {
        "project": data["app_info"]["version"],
        "board": c["board"]["revision"],
        "firmware": c["firmware"]["version"],
    }


def studio_version() -> str:
    return json.loads((ROOT / "studio" / "config" / "version.json").read_text(encoding="utf-8"))["app_info"]["version"]


def status() -> int:
    data = load()
    exp = expected(data)
    info = data["app_info"]
    print(f"Source  {rel(SOURCE)}: проект v{exp['project']}  (release {info['release_date']}, updated {info['last_updated']}, {info.get('developer', '')})")
    print(f"        плата rev {exp['board']}, прошивка {exp['firmware']}, DDS Studio {data['components']['studio']['version']}\n")
    bad = 0
    for label, path, rx, key in trackers():
        found = re.findall(rx, path.read_text(encoding="utf-8")) if path.exists() else []
        if not found:
            print(f"  {label:9} MISSING  {rel(path)}")
            bad += 1
            continue
        ok = all(v == exp[key] for v in found)
        bad += not ok
        print(f"  {label:9} {'OK' if ok else 'DRIFT':8} {rel(path):40} {', '.join(sorted(set(found)))}")
    sv = studio_version()
    ok = sv == data["components"]["studio"]["version"]
    bad += not ok
    print(f"  {'studio':9} {'OK' if ok else 'DRIFT':8} {'studio/config/version.json':40} {sv}")
    print("\nAll tracked files match." if not bad else f"\n{bad} mismatch(es): python tools/update_version.py sync")
    return 1 if bad else 0


def sync(data: dict, dry: bool) -> None:
    exp = expected(data)
    for label, path, rx, key in trackers():
        text = path.read_text(encoding="utf-8")
        new = re.sub(rx, lambda m: m.group(0)[: m.start(1) - m.start(0)] + exp[key] + m.group(0)[m.end(1) - m.start(0):], text)
        if new != text:
            if dry:
                print(f"[DRY RUN] {rel(path)} -> {label} {exp[key]}")
            else:
                path.write_text(new, encoding="utf-8", newline="")
                print(f"Updated {rel(path)} ({label} {exp[key]})")
    data["components"]["studio"]["version"] = studio_version()


def touch(data: dict, release: bool) -> None:
    today = date.today()
    data["app_info"]["last_updated"] = today.isoformat()
    if release:
        data["app_info"]["release_date"] = today.strftime("%d.%m.%Y")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Версии репозитория DDS")
    ap.add_argument("--dry-run", action="store_true")
    sp = ap.add_subparsers(dest="cmd", required=True)
    sp.add_parser("status")
    sp.add_parser("sync")
    b = sp.add_parser("bump"); b.add_argument("part", choices=["patch", "minor", "major"])
    fw = sp.add_parser("firmware"); fw.add_argument("action", choices=["bump", "set"]); fw.add_argument("value")
    bd = sp.add_parser("board"); bd.add_argument("action", choices=["next", "set"]); bd.add_argument("value", nargs="?")
    st = sp.add_parser("studio"); st.add_argument("args", nargs=argparse.REMAINDER)
    a = ap.parse_args(argv)

    if a.cmd == "status":
        return status()
    if a.cmd == "studio":
        r = subprocess.call([sys.executable, str(STUDIO_TOOL), *a.args])
        if r == 0 and not a.dry_run:
            data = load()
            if data["components"]["studio"]["version"] != studio_version():
                data["components"]["studio"]["version"] = studio_version()
                touch(data, release=False)
                save(data, False)
        return r

    data = load()
    if a.cmd == "bump":
        data["app_info"]["version"] = bumped(data["app_info"]["version"], a.part)
        touch(data, release=True)
    elif a.cmd == "firmware":
        cur = data["components"]["firmware"]["version"]
        if a.action == "bump":
            if a.value not in ("patch", "minor", "major"):
                raise SystemExit("firmware bump patch|minor|major")
            data["components"]["firmware"]["version"] = bumped(cur, a.value)
        else:
            semver(a.value)
            data["components"]["firmware"]["version"] = a.value
        touch(data, release=False)
    elif a.cmd == "board":
        cur = data["components"]["board"]["revision"]
        if a.action == "next":
            if cur == "Z":
                raise SystemExit("ревизия Z — последняя буква")
            new = chr(ord(cur) + 1)
        else:
            if not a.value or not re.fullmatch(r"[A-Z]", a.value):
                raise SystemExit("board set <буква A–Z>")
            new = a.value
        data["components"]["board"]["revision"] = new
        touch(data, release=False)
        print(f"Ревизия платы {cur} -> {new}. Дальше по pcb-revision-eco: ECO в docs/design_notes.md, ворота, make_outputs.sh, метка rev{new}-baseline.")
    sync(data, a.dry_run)
    save(data, a.dry_run)
    return 0


if __name__ == "__main__":
    sys.exit(main())
