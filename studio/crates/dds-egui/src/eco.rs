//! «Ревизии платы»: the PCB ECO guide. The web UI embeds `public/eco/index.html`
//! in an iframe; here the same data and the same wizard logic are drawn
//! natively. Keep the tables below in sync with the `<script>` of that page
//! (the `eco_matches_web` test checks the data strings).

use crate::app::Shared;
use crate::theme::{mix, Pal};
use crate::widgets as w;
use eframe::egui::{self, Color32, RichText, Sense, Stroke, Ui};

pub const STAGES: [&str; 7] =
    ["База ЭРИ", "Эталон связей", "Схема", "Посад. место", "Расстановка", "Разводка", "Документы"];

/// (code, name, impact per stage)
pub const TYPES: [(&str, &str, [&str; 7]); 8] = [
    ("T1", "Номинал, допуск, примечание", ["да", "—", "поле", "—", "—", "—", "да"]),
    ("T2", "Замена с тем же корпусом и выводами", ["да", "—", "поле", "—", "—", "—", "да"]),
    ("T3", "Замена с другим корпусом", ["да", "—", "да", "да", "локально", "локально", "да"]),
    ("T4", "Соединения: элементы, цепи", ["да", "да", "да", "если нужно", "локально", "локально", "да"]),
    ("T5", "Геометрия платы", ["—", "—", "—", "—", "да", "да", "да"]),
    ("T6", "Шелкография, обозначения", ["—", "—", "иногда", "—", "—", "—", "иногда"]),
    ("T7", "Только текст документов", ["иногда", "—", "штамп", "—", "—", "—", "да"]),
    ("T8", "Параметры производства", ["—", "—", "—", "—", "—", "иногда", "README изг."]),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    Generator,
    #[default]
    Manual,
    Unknown,
}

pub const MODES: [(Mode, &str); 3] =
    [(Mode::Generator, "Генератор"), (Mode::Manual, "Ручная доработка"), (Mode::Unknown, "Неизвестен")];

/// (title, type index, mode)
pub const PRESETS: [(&str, usize, Mode); 9] = [
    ("Заменить L8 дросселем в другом корпусе", 2, Mode::Manual),
    ("Заменить L8 аналогом в том же корпусе", 1, Mode::Manual),
    ("Изменить номинал R101 (порог тока)", 0, Mode::Manual),
    ("Добавить подтяжку на вход ON_OFF", 3, Mode::Manual),
    ("Изменить контур и крепёжные отверстия", 4, Mode::Manual),
    ("Переставить надписи шелкографии", 5, Mode::Manual),
    ("Исправить наименование в штампе", 6, Mode::Manual),
    ("Сменить покрытие на ENIG, стек", 7, Mode::Manual),
    ("Своя правка", 0, Mode::Unknown),
];

/// (key, what it proves, command)
pub const CMD: [(&str, &str, &str); 7] = [
    (
        "erc",
        "ERC схемы — 0 ошибок, 0 предупреждений",
        "kicad-cli sch erc --severity-all -o reports/ERC.rpt DDS_48_52.kicad_sch",
    ),
    (
        "net",
        "Нетлист текущей схемы",
        "kicad-cli sch export netlist --format kicadxml -o output/netlist/netlist.xml DDS_48_52.kicad_sch",
    ),
    (
        "a",
        "Сверка (а): заданный нетлист ревизии ↔ схема — IDENTICAL",
        "python tools/netcompare.py --expected verification/expected_rev<Y>.json output/netlist/netlist.xml",
    ),
    (
        "b",
        "Сверка (б): прошлая ревизия ↔ схема без изменённых позиций — IDENTICAL",
        "python tools/netcompare.py --ignore-values --refmap verification/refmap_rev<X>_to_rev<Y>.json \\\n  --exclude-a <изменённые поз.> --exclude-b <новые поз.> \\\n  verification/netlist_rev<X>.xml output/netlist/netlist.xml",
    ),
    (
        "drc",
        "DRC со сверкой схемы — 0 нарушений / 0 неразведённых / 0 расхождений",
        "kicad-cli pcb drc --schematic-parity --severity-all -o reports/DRC.rpt DDS_48_52.kicad_pcb",
    ),
    ("out", "Пересборка документов и файлов производства (ручные правки сохраняются)", "sh tools/make_outputs.sh ."),
    (
        "diff",
        "Границы ревизии: изменилось только то, что в ECO",
        "git diff --stat && git diff output/bom/BOM.csv output/netlist/netlist.xml",
    ),
];

/// Gates per change type (indices into TYPES order).
pub const GATES: [&[&str]; 8] = [
    &["net", "b", "diff", "out"],
    &["net", "erc", "b", "diff", "out"],
    &["erc", "net", "a", "b", "drc", "out"],
    &["erc", "net", "a", "b", "drc", "out"],
    &["drc", "out"],
    &["drc", "out"],
    &["out", "diff"],
    &["out"],
];

pub const DOCS: [&str; 8] = [
    "BOM.csv — строка позиции; ПЭ3 — номинал; Э3 PDF и Visio — надпись. Gerber и CPL по сути не меняются.",
    "BOM.csv и BOM JLCPCB; ПЭ3 — наименование (тип, ТУ или код заказа); Э3 — надпись; README изготовителя — монтируемые/не монтируемые. Gerber по сути не меняется.",
    "Нетлист (посадочное место), BOM, ПЭ3, Э3; PDF платы и сборочный вид; Gerber и сверловка (медь, маска, паста, шелкография); CPL — координаты и поворот; README изготовителя.",
    "Нетлист и эталоны ревизии, Э3, ПЭ3, BOM; PDF платы, Gerber, CPL — в зоне правки.",
    "PDF платы, Gerber, сверловка, сборочный вид, размер в тексте заказа и README изготовителя.",
    "Gerber (шелкография), PDF платы (F.Silkscreen, F.Fab); при переименовании — Э3, ПЭ3, BOM через карту «старое → новое».",
    "Э3, ПЭ3, Visio — штампы и наименования; лист регистрации изменений в ПЭ3.",
    "README изготовителя, текст заказа RU/EN (docs/order_params.json); при смене стека — пересчёт импеданса и ширин ВЧ-линий.",
];

/// (phase, result, gate)
pub const PHASES: [(&str, &str, &str); 10] = [
    (
        "Среда и каркас",
        "Каталоги, env.sh, git, проектные библиотеки",
        "kicad-cli --version; import pcbnew в Python KiCad",
    ),
    (
        "ТЗ и архитектура",
        "Блок-схема, бюджеты питания и тепла, профиль, изготовитель, стек",
        "Каждое требование ТЗ → узел схемы → способ проверки",
    ),
    (
        "Выбор ЭРИ",
        "База компонентов tools/dds_parts.py, расхождения с ТУ в design_notes.md",
        "У каждой позиции: тип, ТУ или код заказа, корпус, источник, статус",
    ),
    (
        "УГО и посадочные места",
        "lib/*.kicad_sym, lib/*.pretty, УГО ГОСТ кодом",
        "Каждое место привязано к чертежу корпуса или помечено _PLACEHOLDER",
    ),
    ("Схема", "Листы .kicad_sch, нетлист", "ERC 0/0; сверка с заданным нетлистом — IDENTICAL"),
    (
        "Плата: контур, стек, расстановка",
        ".kicad_pcb без трасс, правила изготовителя",
        "Все компоненты в контуре; нет пересечений courtyard",
    ),
    (
        "Разводка",
        "ВЧ кодом, остальное Freerouting, доводка, чистка; Konnect — точечно",
        "DRC --schematic-parity: 0 / 0 / 0",
    ),
    (
        "Документы",
        "PDF, BOM, нетлист; Э3 и ПЭ3 — только make_outputs.sh",
        "Схема = BOM = ПЭ3 = CPL (+DNP); штампы согласованы; нет «skipped»",
    ),
    (
        "Выдача в производство",
        "Gerber, сверловка, BOM/CPL, README и текст заказа",
        "Gerber просмотрен; открытые пункты закрыты или приняты",
    ),
    ("Фиксация", "Метка rev<X>-baseline, история, уроки", "Метка стоит на коммите с чистой сборкой"),
];

/// (symptom, cause, what to do)
pub const PITS: [(&str, &str, &str); 7] = [
    (
        "Повторный проход по трассам в одном процессе даёт мусор",
        "Итераторы SWIG в pcbnew не повторно входимы",
        "Каждый шаг чистки — отдельный процесс, циклом до кода 0",
    ),
    (
        "Нетлист пишет D1A вместо D1.1, пропала рамка",
        "pcbnew перезаписал .kicad_pro без секции схемы",
        "После скрипта на pcbnew — pro_merge.py, потом DRC",
    ),
    (
        "Konnect: «нет соединения»",
        "Открыт только менеджер проектов или выключен API",
        "Открыть PCB Editor с платой; Preferences → Plugins → Enable KiCad API",
    ),
    (
        "Правки через Konnect пропали",
        "После них запускался build_all.sh или route_all.sh",
        "Правки через MCP — ручная доработка: записать режим, генераторы не запускать",
    ),
    (
        "Сборка документов «прошла», а Word/Visio нет",
        "Не установлены python-docx, vsdx, pillow",
        "Проверять вывод на «skipped»",
    ),
    (
        "Комплект с содержанием ревизии G, а в именах F",
        "Имена revF зашиты в make_outputs.sh",
        "Сменить имена и букву ревизии в штампах и шелкографии",
    ),
    (
        "Конфликт .kicad_pcb после двух машин",
        "Такие файлы вручную не сливаются",
        "git pull → работа → ERC/DRC → commit → push; один файл — одна машина",
    ),
];

/// Pages of the guide: (title, hint, group). Same order as the page's sidebar.
pub const PAGES: [(&str, &str, &str); 7] = [
    ("Мастер изменения", "Маршрут под вашу правку", "Изменение"),
    ("Конвейер", "Фазы 0–9 и их ворота", "Процесс"),
    ("Режимы", "Генератор или ручная доработка", "Процесс"),
    ("Матрица влияния", "T1–T8: что перестраивается", "Изменение"),
    ("Ворота и команды", "ERC, сверки, DRC, сборка", "Изменение"),
    ("Кто что делает", "Вы, Claude, Konnect, скрипты", "Справка"),
    ("Грабли", "Симптом → причина → что делать", "Справка"),
];

/// Who does a step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Who {
    You,
    Claude,
    Konnect,
    Stop,
}

impl Who {
    fn label(self) -> &'static str {
        match self {
            Who::You => "вы",
            Who::Claude => "Claude",
            Who::Konnect => "Konnect",
            Who::Stop => "стоп",
        }
    }
    fn color(self, p: &Pal) -> Color32 {
        match self {
            Who::You => p.refc,
            Who::Claude => p.dig,
            Who::Konnect => p.clk,
            Who::Stop => p.err,
        }
    }
}

pub struct EcoState {
    pub page: usize,
    pub ty: usize,
    pub mode: Mode,
    pub konnect: bool,
    pub preset: usize,
    pub phase: usize,
    /// Command key just copied and when (for the «Скопировано» label).
    copied: Option<(&'static str, f64)>,
}

impl Default for EcoState {
    fn default() -> Self {
        EcoState { page: 0, ty: 2, mode: Mode::Manual, konnect: true, preset: 0, phase: 6, copied: None }
    }
}

/// Route of the change: (who, heading, text). Port of `steps()` in the page script.
pub fn steps(ty: usize, mode: Mode, konnect: bool) -> Vec<(Who, String, String)> {
    let t = TYPES[ty].0;
    let k = konnect;
    let edit = if k { Who::Konnect } else { Who::You };
    let mut out: Vec<(Who, String, String)> = vec![];
    let mut add = |w: Who, h: &str, p: &str| out.push((w, h.to_string(), p.to_string()));
    add(Who::Claude, "Подготовка", "git fetch и status, найти метку rev<X>-baseline, ветка rev<Y>, python tools/update_version.py board next --yes, опорная сборка даёт записанные числа ERC/DRC/компонентов.");
    add(
        Who::Claude,
        "Запись ECO",
        &format!(
            "Раздел «Ревизия <Y>» в design_notes.md: основание, тип {}, позиции, «было → стало», расчёт. Неточное описание — уточнить у вас.",
            t
        ),
    );
    if mode == Mode::Unknown {
        add(
            Who::Stop,
            "Стоп: определить режим",
            "По git log и design_notes.md. Пока режим не записан, генераторы схемы и разводки не запускать.",
        );
        return out;
    }
    let gen = mode == Mode::Generator;
    match t {
        "T1" => {
            if gen {
                add(
                    Who::Claude,
                    "Правка в базе ЭРИ",
                    "Запись в tools/dds_parts.py → сборка только схемы, без расстановки и разводки.",
                );
            } else {
                add(
                    Who::You,
                    "Поле в схеме → F8",
                    "Значение подтянется в плату, трассы сохранятся. Затем синхронизировать базу ЭРИ.",
                );
            }
            add(Who::Claude, "Пересчитать расчёт", "Расчёт, из которого получен номинал, — в design_notes.md.");
        }
        "T2" => {
            add(Who::Claude, "Проверка совместимости", "Питание, пороги входов ТТЛ/КМОП, выходные токи, частота, температура, рассеяние — таблица «было → стало».");
            if gen {
                add(Who::Claude, "Правка в базе ЭРИ", "Тип, ТУ, наименование для ПЭ3 → сборка схемы.");
            } else {
                add(
                    Who::You,
                    "Поле в схеме → F8",
                    "Посадочное место прежнее — разводку не трогаем. Базу ЭРИ синхронизировать.",
                );
            }
        }
        "T3" => {
            add(
                Who::Claude,
                "Новое посадочное место",
                "По чертежу корпуса из ТУ; нет чертежа — _PLACEHOLDER и открытый пункт.",
            );
            if gen {
                add(Who::Claude, "Расстановка генератором", "Правка базы и генератора посадочных мест → build_all.sh.");
                add(
                    Who::Stop,
                    "route_all.sh — вся разводка заново",
                    "Если разводка ценна — сначала копия pcb_route/rev<X>_routed.kicad_pcb и ваше согласие.",
                );
            } else {
                add(
                    Who::You,
                    "F8 с заменой посадочного места",
                    "В PCB Editor: Update PCB from Schematic, отметить замену посадочных мест.",
                );
                let how = if k {
                    "Через Konnect: только цепи выводов элемента. "
                } else {
                    "ripnet.py или вручную: только цепи выводов. "
                };
                add(edit, "Снять трассы цепей элемента", &format!("{}Остальную плату не трогать.", how));
                let tail = if k {
                    "Правки видны сразу, Ctrl+Z работает."
                } else {
                    "Затем zero_len.py до кода 0."
                };
                add(
                    edit,
                    "Поставить корпус и доразвести",
                    &format!("Развязка рядом, петля DC/DC минимальной площади, тепловые полигоны. {}", tail),
                );
                add(Who::You, "Глазами: петля, тепло, полярность", "DRC не знает, что петля коммутации длинная.");
            }
        }
        "T4" => {
            add(Who::Claude, "Описание правок ревизии", "Удаляемые и новые позиции, изменённые цепи; ключи новых элементов ≥ 100. Эталоны пишет генератор, не руками.");
            if gen {
                add(Who::Claude, "Сборка схемы, расстановка, разводка", "build_all.sh со сверками → route_all.sh.");
            } else {
                add(Who::You, "Схема в KiCad → F8", "Изменить схему, обновить плату.");
                add(
                    edit,
                    "Доразвести изменённые цепи",
                    if k {
                        "Через Konnect — только новые и изменённые цепи."
                    } else {
                        "minirouter.py или вручную."
                    },
                );
                add(
                    Who::Claude,
                    "Привести данные к факту",
                    "Описание правок и база ЭРИ = фактическая схема; сгенерировать эталоны.",
                );
            }
        }
        "T5" => {
            if gen {
                add(
                    Who::Claude,
                    "Константы контура и расстановки",
                    "Сборка и разводка; ВЧ-трассы по координатам пересчитать.",
                );
            } else {
                add(edit, "Edge.Cuts и позиции", "Перезалить зоны, сшивку, тепловые полигоны; проверить ВЧ-линии.");
                add(Who::You, "Корпус и разъёмы", "Зазор медь–край, крепёж, положение разъёмов относительно корпуса.");
            }
        }
        "T6" => {
            add(
                Who::Claude,
                "Обозначения из схемы",
                "Переименование — через карту «старое → новое», чтобы сверки работали.",
            );
            if gen {
                add(Who::Claude, "Расстановка надписей", "silk_place.py — только шелкография, медь не трогает.");
            } else {
                let via = if k { "Через Konnect или" } else { "Через" };
                add(edit, "Расстановка надписей", &format!("{} silk_place.py — только шелкография.", via));
            }
            add(
                Who::You,
                "Проверка в PDF платы",
                "F.Silkscreen и F.Fab: читаемость, метки первого вывода, буква ревизии.",
            );
        }
        "T7" => {
            add(Who::Claude, "Переменные листа и платы", "${TITLE}, ${COMMENT1..9}, параметры генератора ПЭ3 — в одном месте. Фамилии — только по данным заказчика.");
            add(Who::Claude, "Лист регистрации изменений", "Если КД выпущена с литерой — извещение по ГОСТ 2.503.");
        }
        _ => {
            add(Who::Claude, "Метаданные платы и параметры заказа", "docs/order_params.json, README изготовителя; смена стека → пересчёт импеданса (это уже T5/T4 для трасс).");
        }
    }
    add(Who::Claude, "Ворота", "После каждого типа правки, а не один раз в конце. Не ноль — вернуться к правке.");
    add(Who::Claude, "Документы", "sh tools/make_outputs.sh . — бинарные файлы отдельным коммитом «Rebuild outputs».");
    add(
        Who::Claude,
        "Фиксация",
        "design_notes.md, строка истории в README, коммит «Rev <Y>: …», метка rev<Y>-baseline.",
    );
    add(Who::You, "Отправка", "git push origin main и git push nas main; на Mac — git pull.");
    out
}

/// Warnings for the chosen type and mode: ("err" | "warn" | "ok", text). Port of `alerts()`.
pub fn alerts(ty: usize, mode: Mode, konnect: bool) -> Vec<(&'static str, &'static str)> {
    let t = TYPES[ty].0;
    let mut a = vec![];
    if mode == Mode::Unknown {
        a.push(("err", "Режим не определён — генераторы не запускаются. Сначала запись режима в design_notes.md."));
    }
    if mode == Mode::Generator && ["T3", "T4", "T5"].contains(&t) {
        a.push(("err", "В режиме генератора эта правка перестраивает разводку всей платы, ручные доработки теряются."));
    }
    if mode == Mode::Generator && konnect {
        a.push((
            "warn",
            "Konnect в режиме генератора — только чтение и анализ: его правки затрёт следующий build_all.sh.",
        ));
    }
    if mode == Mode::Manual && konnect && ["T3", "T4", "T5", "T6"].contains(&t) {
        a.push(("ok", "Правка точечная: только цепи и область изменения, остальная разводка сохраняется. Konnect правит в открытом PCB Editor, Ctrl+Z работает."));
    }
    if mode == Mode::Manual && !konnect && ["T3", "T4", "T5"].contains(&t) {
        a.push(("warn", "Без Konnect точечная правка — руками в KiCad или скриптами ripnet / minirouter / drc_fix, каждый отдельным процессом."));
    }
    if t == "T4" || t == "T6" {
        a.push(("warn", "Схема меняется: Konnect пишет .kicad_sch напрямую (в KiCad 10 нет IPC для схем) — после этого обе сверки и ERC обязательны."));
    }
    a
}

/// Stage cell for the wizard chain: (text, class) where class is "" | "on" | "big".
pub fn stage(ty: usize, mode: Mode, i: usize) -> (&'static str, &'static str) {
    let v = TYPES[ty].2[i];
    if mode == Mode::Generator && (i == 4 || i == 5) && (v == "локально" || v == "да") {
        return ("целиком", "big");
    }
    (v, if v == "—" { "" } else { "on" })
}

// ───────────────────────────── drawing ─────────────────────────────

/// Text with `code` spans in monospace and **bold** spans.
fn rich(ui: &mut Ui, p: &Pal, text: &str, size: f32, color: Color32) {
    let mut job = egui::text::LayoutJob::default();
    let prop = egui::FontId::proportional(size);
    let mono = egui::FontId::monospace(size - 0.5);
    let mut code = false;
    let mut bold = false;
    let mut buf = String::new();
    let flush = |job: &mut egui::text::LayoutJob, buf: &mut String, code: bool, bold: bool| {
        if buf.is_empty() {
            return;
        }
        let fmt = egui::TextFormat {
            font_id: if code { mono.clone() } else { prop.clone() },
            color: if bold { p.fg } else { color },
            background: if code { p.panel3 } else { Color32::TRANSPARENT },
            ..Default::default()
        };
        job.append(buf, 0.0, fmt);
        buf.clear();
    };
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '`' {
            flush(&mut job, &mut buf, code, bold);
            code = !code;
        } else if c == '*' && chars.peek() == Some(&'*') {
            chars.next();
            flush(&mut job, &mut buf, code, bold);
            bold = !bold;
        } else {
            buf.push(c);
        }
    }
    flush(&mut job, &mut buf, code, bold);
    job.wrap.max_width = ui.available_width();
    ui.add(egui::Label::new(job).wrap());
}

fn bullets(ui: &mut Ui, p: &Pal, items: &[&str]) {
    for it in items {
        ui.horizontal_top(|ui| {
            ui.label(RichText::new("•").color(p.muted));
            rich(ui, p, it, 13.0, p.fg2);
        });
    }
}

fn boxed<R>(ui: &mut Ui, p: &Pal, accent: Option<(Color32, bool)>, body: impl FnOnce(&mut Ui) -> R) -> R {
    // accent: (colour, top border) — otherwise left border
    let r = egui::Frame::new()
        .fill(p.panel2)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(12)
        .inner_margin(egui::Margin {
            left: if accent.is_some_and(|a| !a.1) { 16 } else { 14 },
            right: 14,
            top: 14,
            bottom: 14,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui)
        });
    if let Some((c, top)) = accent {
        let rr = r.response.rect;
        let bar = if top {
            egui::Rect::from_min_size(rr.min + egui::vec2(6.0, 0.0), egui::vec2(rr.width() - 12.0, 3.0))
        } else {
            egui::Rect::from_min_size(rr.min + egui::vec2(0.0, 6.0), egui::vec2(4.0, rr.height() - 12.0))
        };
        ui.painter().rect_filled(bar, 2.0, c);
    }
    r.inner
}

fn cmd_box(ui: &mut Ui, sh: &Shared, st: &mut EcoState, key: &'static str) {
    let p = &sh.pal;
    let Some((_, what, cmd)) = CMD.iter().find(|c| c.0 == key) else { return };
    egui::Frame::new()
        .fill(p.panel2)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(*what).size(12.0).color(p.muted));
            ui.horizontal(|ui| {
                let copied = st.copied.is_some_and(|(k, t)| k == key && sh.time - t < 1.5);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.add(w::small(p, if copied { "Скопировано" } else { "Копировать" }, w::Btn::Subtle)).clicked()
                    {
                        ui.ctx().copy_text(cmd.to_string());
                        st.copied = Some((key, sh.time));
                    }
                    if copied {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        ui.add(egui::Label::new(RichText::new(*cmd).monospace().size(12.0).color(p.fg)).wrap());
                    });
                });
            });
        });
}

fn lead(ui: &mut Ui, p: &Pal, title: &str, text: &str) {
    w::card(ui, p, "", "", |ui| {
        ui.label(RichText::new(title).size(17.0).strong());
        ui.add_space(2.0);
        rich(ui, p, text, 13.5, p.fg2);
    });
}

pub fn show(ui: &mut Ui, sh: &Shared, st: &mut EcoState) {
    let p = sh.pal;
    w::page_header(
        ui,
        &p,
        "🔧",
        "Ревизии платы",
        "Как вносить исправления в плату DDS 48–52: мастер изменения T1–T8, режимы, ворота, роли и грабли.",
        |_| {},
    );

    // the page's own status bar and navigation
    ui.horizontal_wrapped(|ui| {
        for (t, warn) in [
            ("KiCad 10", false),
            ("Konnect v0.13.0 подключён", false),
            ("Профиль: ГОСТ ЕСКД", false),
            ("Режим: смотреть docs/design_notes.md", true),
            ("Сборка КД: tools/make_outputs.sh", false),
        ] {
            w::pill(ui, &p, t, if warn { p.warn } else { p.ok }, true);
        }
    });
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        let mut group = "";
        for (i, (title, hint, g)) in PAGES.iter().enumerate() {
            if *g != group {
                group = g;
                ui.label(RichText::new(g.to_uppercase()).size(10.0).color(p.muted).strong());
            }
            let on = st.page == i;
            let b = egui::Button::new(RichText::new(*title).size(12.5).color(if on { p.accent } else { p.fg2 }))
                .fill(if on { p.panel3 } else { p.panel })
                .stroke(Stroke::new(1.0, if on { p.line2 } else { p.line }))
                .corner_radius(8)
                .min_size(egui::vec2(0.0, 28.0));
            if ui.add(b).on_hover_text(*hint).clicked() {
                st.page = i;
            }
        }
    });
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for (c, t) in [
            (p.refc, "вы, в KiCad"),
            (p.dig, "Claude"),
            (p.clk, "Konnect (MCP)"),
            (p.sig, "затронуто правкой"),
            (p.err, "стоп / перестраивается целиком"),
        ] {
            let (r, _) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), Sense::hover());
            ui.painter().rect_filled(r, 3.0, c);
            ui.label(RichText::new(t).size(12.0).color(p.fg2));
            ui.add_space(8.0);
        }
    });
    ui.add_space(10.0);
    ui.spacing_mut().item_spacing.y = 14.0;

    match st.page {
        0 => wizard(ui, sh, st),
        1 => pipeline(ui, &p, st),
        2 => modes(ui, &p),
        3 => matrix(ui, &p, st),
        4 => {
            lead(
                ui,
                &p,
                "Ворота ревизии и команды",
                "Результат сообщается числами. Команда, которая не запускалась, — «не проверено», а не «всё хорошо». Сверку не «лечат» расширением исключений, пока не понятна причина.",
            );
            w::card(ui, &p, "", "", |ui| {
                for (k, _, _) in CMD {
                    cmd_box(ui, sh, st, k);
                }
            });
        }
        5 => roles(ui, &p),
        _ => pitfalls(ui, &p),
    }
}

fn wizard(ui: &mut Ui, sh: &Shared, st: &mut EcoState) {
    let p = sh.pal;
    lead(
        ui,
        &p,
        "Мастер изменения",
        "Выберите, что меняете и в каком режиме проект. Справа — маршрут правки, зона перестройки, ворота и команды. Это та же логика, что в навыке `pcb-revision-eco`.",
    );
    w::two_cols(
        ui,
        380.0,
        true,
        st,
        |ui, st| {
            w::card(ui, &p, "", "", |ui| {
                let (code, name, _) = TYPES[st.ty];
                ui.label(RichText::new(format!("{} · {}", code, name)).size(14.0).strong());
                let sub = format!(
                    "Режим: {}{}",
                    MODES.iter().find(|m| m.0 == st.mode).map(|m| m.1).unwrap_or(""),
                    if st.konnect { " · Konnect подключён" } else { " · без MCP" }
                );
                ui.label(RichText::new(sub).size(12.5).color(p.muted));
                ui.add_space(4.0);
                // chain of stages
                let n = 7.0;
                let gap = 6.0;
                let cw = ((ui.available_width() - gap * (n - 1.0)) / n).max(60.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                    for (i, name) in STAGES.iter().enumerate() {
                        let (v, cls) = stage(st.ty, st.mode, i);
                        let (fill, stroke, head) = match cls {
                            "big" => (mix(p.err, 10.0, p.panel), p.err, p.fg2),
                            "on" => (mix(p.sig, 12.0, p.panel), p.sig, p.fg2),
                            _ => (p.panel2, p.line, p.muted),
                        };
                        egui::Frame::new()
                            .fill(fill)
                            .stroke(Stroke::new(1.0, stroke))
                            .corner_radius(8)
                            .inner_margin(8)
                            .show(ui, |ui| {
                                ui.set_width(cw - 18.0);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 4.0;
                                    ui.set_min_height(40.0);
                                    ui.add(egui::Label::new(RichText::new(*name).size(11.5).color(head)).truncate());
                                    ui.label(RichText::new(v).monospace().size(12.5));
                                });
                            });
                    }
                });
                for (cls, text) in alerts(st.ty, st.mode, st.konnect) {
                    let c = match cls {
                        "err" => p.err,
                        "warn" => p.warn,
                        _ => p.ok,
                    };
                    egui::Frame::new()
                        .fill(mix(c, 8.0, p.panel))
                        .stroke(Stroke::new(1.0, mix(c, 50.0, p.line)))
                        .corner_radius(10)
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.add(egui::Label::new(RichText::new(text).size(13.0)).wrap());
                        });
                }
                for (i, (who, h, text)) in steps(st.ty, st.mode, st.konnect).iter().enumerate() {
                    let c = who.color(&p);
                    let r = egui::Frame::new()
                        .fill(p.panel2)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(10)
                        .inner_margin(egui::Margin { left: 14, right: 12, top: 10, bottom: 10 })
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_top(|ui| {
                                let (nr, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::hover());
                                ui.painter().rect_filled(nr, 7.0, p.panel3);
                                ui.painter().text(
                                    nr.center(),
                                    egui::Align2::CENTER_CENTER,
                                    (i + 1).to_string(),
                                    egui::FontId::monospace(12.0),
                                    p.fg2,
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                    egui::Frame::new()
                                        .stroke(Stroke::new(1.0, p.line))
                                        .corner_radius(10)
                                        .inner_margin(egui::Margin::symmetric(8, 2))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(who.label()).size(11.0).color(c));
                                        });
                                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.label(RichText::new(h).strong().size(13.0));
                                        ui.add(egui::Label::new(RichText::new(text).size(13.0).color(p.fg2)).wrap());
                                    });
                                });
                            });
                        });
                    let rr = r.response.rect;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(rr.min, egui::vec2(3.0, rr.height())),
                        egui::CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 },
                        c,
                    );
                }
                w::section(ui, &p, "Ворота этой правки");
                if st.mode == Mode::Unknown {
                    ui.label(RichText::new("Сначала определить режим.").size(12.5).color(p.muted));
                } else {
                    for k in GATES[st.ty] {
                        cmd_box(ui, sh, st, k);
                    }
                }
                w::section(ui, &p, "Что изменится в документах");
                ui.add(egui::Label::new(RichText::new(DOCS[st.ty]).size(13.0).color(p.fg2)).wrap());
            });
        },
        |ui, st| {
            w::card(ui, &p, "", "", |ui| {
                w::section(ui, &p, "Готовые сценарии");
                let opts: Vec<(usize, String)> =
                    PRESETS.iter().enumerate().map(|(i, x)| (i, x.0.to_string())).collect();
                if let Some(i) = w::select(ui, "eco-preset", &st.preset, &opts, ui.available_width() - 8.0) {
                    st.preset = i;
                    st.ty = PRESETS[i].1;
                    st.mode = PRESETS[i].2;
                }
                w::section(ui, &p, "Тип изменения");
                for (i, (code, name, _)) in TYPES.iter().enumerate() {
                    let on = st.ty == i;
                    let r = egui::Frame::new()
                        .fill(if on { p.accent } else { p.panel2 })
                        .stroke(Stroke::new(1.0, if on { p.accent } else { p.line }))
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let fg = if on { p.accent_fg } else { p.fg };
                                ui.add_sized(
                                    [30.0, 16.0],
                                    egui::Label::new(RichText::new(*code).monospace().size(12.5).color(fg)),
                                );
                                ui.label(RichText::new(*name).size(12.5).color(fg));
                            });
                        })
                        .response
                        .interact(Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if r.clicked() {
                        st.ty = i;
                    }
                }
                w::section(ui, &p, "Режим проекта");
                if let Some(m) = w::segmented(ui, &p, st.mode, &MODES) {
                    st.mode = m;
                }
                w::section(ui, &p, "MCP");
                ui.horizontal(|ui| {
                    w::toggle_tone(ui, &p, &mut st.konnect, p.clk);
                    ui.label(RichText::new("Konnect подключён, KiCad открыт").size(13.0));
                });
            });
        },
    );
}

fn pipeline(ui: &mut Ui, p: &Pal, st: &mut EcoState) {
    w::card(ui, p, "", "", |ui| {
        ui.label(RichText::new("Конвейер разработки платы").size(17.0).strong());
        rich(ui, p, "Цепочка однонаправленная: база компонентов и заданные соединения → генераторы → проект KiCad → документы. Документы правят только через источник. В следующую фазу переходят, только когда ворота пройдены командой.", 13.5, p.fg2);
        ui.add_space(8.0);
        w::auto_grid(ui, 170.0, PHASES.len(), |ui, i| {
            let on = st.phase == i;
            let r = egui::Frame::new()
                .fill(if on { mix(p.accent, 9.0, p.panel) } else { p.panel2 })
                .stroke(Stroke::new(1.0, if on { p.accent } else { p.line }))
                .corner_radius(10)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(46.0);
                    ui.label(RichText::new(format!("Фаза {}", i)).monospace().size(12.0).color(p.accent));
                    ui.add(egui::Label::new(RichText::new(PHASES[i].0).size(12.5)).wrap());
                })
                .response
                .interact(Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if r.clicked() {
                st.phase = i;
            }
        });
        ui.add_space(8.0);
        let (_, res, gate) = PHASES[st.phase];
        w::cols(ui, 2, 260.0, |ui, i| {
            let (k, v) = if i == 0 {
                ("РЕЗУЛЬТАТ", res)
            } else {
                ("ВОРОТА (ПРОВЕРЯЕТСЯ КОМАНДОЙ)", gate)
            };
            boxed(ui, p, None, |ui| {
                ui.label(RichText::new(k).size(11.5).color(p.muted));
                ui.add(egui::Label::new(RichText::new(v).size(13.5)).wrap());
            });
        });
    });
}

fn modes(ui: &mut Ui, p: &Pal) {
    lead(
        ui,
        p,
        "Два режима и порог между ними",
        "Режим определяет, какими инструментами можно править. Переход из режима генератора в ручную доработку необратим без потери правок: момент перехода записывается в `docs/design_notes.md` и отмечается git-меткой.",
    );
    let items: [(&str, &str, Color32, &[&str]); 3] = [
        (
            "Режим генератора",
            "Схема и плата строятся скриптами заново",
            p.sig,
            &[
                "Правка — в данных: база ЭРИ, описание правок, расстановка, генератор посадочных мест.",
                "`build_all.sh` и `route_all.sh` перестраивают всё.",
                "Воспроизводимо, но **разводка переделывается целиком**.",
                "Konnect — только чтение и анализ.",
            ],
        ),
        (
            "Режим ручной доработки",
            "Плата правилась руками, разводка ценна",
            p.refc,
            &[
                "Генераторы схемы и разводки **не запускать**.",
                "Правки — в KiCad, через Konnect или точечными скриптами `ripnet`, `minirouter`, `drc_fix`.",
                "Разрешена только сборка документов `make_outputs.sh`.",
                "После правки синхронизировать базу и эталоны, иначе следующий генератор откатит её.",
            ],
        ),
        (
            "Режим неизвестен",
            "Не записан в design_notes.md",
            p.err,
            &[
                "Ничего не перестраивать.",
                "Определить по `git log`: были ли коммиты `.kicad_pcb` без коммитов генераторов.",
                "Спросить вас и записать решение.",
            ],
        ),
    ];
    w::cols(ui, 3, 240.0, |ui, i| {
        let (t, sub, c, list) = items[i];
        boxed(ui, p, Some((c, true)), |ui| {
            ui.label(RichText::new(t).size(14.0).strong());
            ui.label(RichText::new(sub).size(12.5).color(p.muted));
            ui.add_space(6.0);
            bullets(ui, p, list);
        });
    });
}

fn matrix(ui: &mut Ui, p: &Pal, st: &mut EcoState) {
    w::card(ui, p, "", "", |ui| {
        ui.label(RichText::new("Матрица влияния T1–T8").size(17.0).strong());
        rich(ui, p, "Каждый тип правки затрагивает только часть цепочки. Дальше этой зоны не ходят: разводка, не затронутая изменением, не пересчитывается. Нажмите на строку, чтобы открыть её в мастере. Несколько типов выполняются в порядке T4 → T3 → T2/T1 → T5 → T6 → T7 → T8.", 13.5, p.fg2);
        ui.add_space(8.0);
        let first = (ui.available_width() * 0.28).max(220.0);
        let cw = ((ui.available_width() - first) / 7.0).max(70.0);
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("eco-matrix").spacing([0.0, 0.0]).min_row_height(30.0).show(ui, |ui| {
                ui.add_sized([first, 26.0], egui::Label::new(RichText::new("Тип").size(11.5).color(p.muted)));
                for s in STAGES {
                    ui.add_sized([cw, 26.0], egui::Label::new(RichText::new(s).size(11.5).color(p.muted)));
                }
                ui.end_row();
                for (i, (code, name, m)) in TYPES.iter().enumerate() {
                    let row_top = ui.cursor().min.y;
                    let mut clicked = false;
                    let on = st.ty == i;
                    let r = ui
                        .allocate_ui_with_layout(
                            egui::vec2(first, 30.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.set_width(first);
                                ui.label(RichText::new(*code).monospace().size(12.5));
                                ui.add(egui::Label::new(RichText::new(*name).size(13.0)).truncate());
                            },
                        )
                        .response;
                    clicked |= r.interact(Sense::click()).clicked();
                    for v in m {
                        let c = if *v == "—" { p.muted } else { p.sig };
                        let r = ui.add_sized(
                            [cw, 30.0],
                            egui::Label::new(RichText::new(*v).monospace().size(12.0).color(c)).sense(Sense::click()),
                        );
                        clicked |= r.clicked();
                    }
                    let _ = row_top;
                    if on {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(r.rect.min - egui::vec2(8.0, 2.0), egui::vec2(3.0, 30.0)),
                            1.0,
                            p.accent,
                        );
                    }
                    ui.end_row();
                    if clicked {
                        st.ty = i;
                        st.page = 0;
                    }
                }
            });
        });
    });
}

fn roles(ui: &mut Ui, p: &Pal) {
    lead(
        ui,
        p,
        "Кто что делает",
        "Konnect правит то, что открыто в KiCad, сейчас. Скрипты строят воспроизводимо. Claude ведёт процесс и проверяет. Вы принимаете решения и делаете то, что есть только в интерфейсе KiCad.",
    );
    let items: [(&str, &str, Color32, &[&str]); 5] = [
        (
            "Вы",
            "решения и интерфейс KiCad",
            p.refc,
            &[
                "Выбор ЭРИ и подтверждение ECO",
                "F8 «Update PCB from Schematic»",
                "Визуальная проверка петель DC/DC, ВЧ, шелкографии",
                "`git push` в origin и nas",
            ],
        ),
        (
            "Claude",
            "процесс, расчёты, проверки",
            p.dig,
            &[
                "Записи ECO, расчёт по ТУ, таблица «было → стало»",
                "Посадочные места по чертежу",
                "Запуск ворот и разбор отчётов",
                "`make_outputs.sh`, design_notes, коммит, метка",
            ],
        ),
        (
            "Konnect",
            "MCP, KiCad 10, IPC API",
            p.clk,
            &[
                "Чтение платы: компоненты, цепи, трассы",
                "Точечные правки в открытом PCB Editor, Ctrl+Z работает",
                "Схему пишет прямо в `.kicad_sch`: в KiCad 10 нет IPC для схем",
                "Экспорт — только для просмотра",
            ],
        ),
        (
            "Скрипты tools/",
            "воспроизводимая сборка",
            p.sig,
            &[
                "`build_all.sh`, `route_all.sh` — затирают ручные правки",
                "`make_outputs.sh` — сохраняет их",
                "`netcompare.py`, `pro_merge.py`",
                "`ripnet`, `minirouter`, `drc_fix`, `silk_place`",
            ],
        ),
        (
            "GitHub Actions",
            "DDS Studio и прошивка",
            p.ctl,
            &[
                "Push: тесты прошивки и студии",
                "Ручной запуск CI: установщики macOS и Windows",
                "Плату и КД не собирает",
            ],
        ),
    ];
    w::auto_grid(ui, 220.0, items.len(), |ui, i| {
        let (t, sub, c, list) = items[i];
        boxed(ui, p, Some((c, false)), |ui| {
            ui.set_min_height(170.0);
            ui.label(RichText::new(t).size(14.0).strong());
            ui.label(RichText::new(sub).size(12.5).color(p.muted));
            ui.add_space(6.0);
            bullets(ui, p, list);
        });
    });
}

fn pitfalls(ui: &mut Ui, p: &Pal) {
    w::card(ui, p, "", "", |ui| {
        ui.label(RichText::new("Грабли").size(17.0).strong());
        ui.label(RichText::new("Проверено на ревизиях B–F.").size(13.5).color(p.fg2));
        ui.add_space(8.0);
        for (a, b, c) in PITS {
            boxed(ui, p, None, |ui| {
                w::cols(ui, 3, 200.0, |ui, i| {
                    let (k, v) = [("СИМПТОМ", a), ("ПРИЧИНА", b), ("ЧТО ДЕЛАТЬ", c)][i];
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(RichText::new(k).size(11.0).color(p.muted));
                    ui.add(egui::Label::new(RichText::new(v).size(13.0)).wrap());
                });
            });
            ui.add_space(-4.0);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> String {
        std::fs::read_to_string(format!("{}/../../public/eco/index.html", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn eco_matches_web() {
        let html = page();
        let mut all: Vec<&str> = vec![];
        all.extend(STAGES);
        all.extend(TYPES.iter().map(|t| t.1));
        all.extend(TYPES.iter().flat_map(|t| t.2));
        all.extend(PRESETS.iter().map(|p| p.0));
        all.extend(CMD.iter().flat_map(|c| [c.1]));
        all.extend(DOCS);
        all.extend(PHASES.iter().flat_map(|p| [p.0, p.1, p.2]));
        all.extend(PITS.iter().flat_map(|p| [p.0, p.1, p.2]));
        all.extend(PAGES.iter().flat_map(|p| [p.0, p.1]));
        for (_, _, c) in CMD {
            all.push(c.split(" \\").next().unwrap());
        }
        for s in all {
            assert!(html.contains(s), "public/eco/index.html lost: {}", s);
        }
    }

    #[test]
    fn wizard_logic_matches_the_page() {
        // unknown mode stops after the ECO record
        let s = steps(2, Mode::Unknown, true);
        assert_eq!(s.len(), 3);
        assert_eq!(s[2].0, Who::Stop);
        // T3 by hand with Konnect: Konnect re-routes, the user checks
        let s = steps(2, Mode::Manual, true);
        assert!(s.iter().any(|x| x.0 == Who::Konnect && x.1 == "Снять трассы цепей элемента"));
        assert_eq!(s.last().unwrap().1, "Отправка");
        // generator mode rebuilds placement and routing completely
        assert_eq!(stage(2, Mode::Generator, 5), ("целиком", "big"));
        assert_eq!(stage(0, Mode::Manual, 1), ("—", ""));
        assert!(alerts(2, Mode::Generator, true).iter().any(|a| a.0 == "err"));
        assert!(alerts(3, Mode::Manual, true).iter().any(|a| a.0 == "ok"));
    }
}
