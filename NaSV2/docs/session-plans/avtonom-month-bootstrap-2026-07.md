# AVTONOM · Month-Bootstrap prompt — 2026-07 (M2)

> Seeded by RETRO-2026-06.md (`docs/session-plans/RETRO-2026-06.md`).
> Phase A PF1 should read that retro FIRST — its §7 (Next-month
> proposals) is the input for the new G1..G5.
>
> Also load the MPD-001 commerce + CRM pivot — `docs/governance/master-plan-diffs/MPD-001-commerce-crm-pivot.md`
> — and weave commerce / CRM threads into Phase B per MPD-001 §3
> revised theme for M2.
>
> One-time bootstrap. Run this in Claude Code at
> `F:\Users\a\Documents\_DEV\Tran\ES\barbie\AX\NaSV2`.
>
> The bootstrap produces, in one session:
>   1. **AUDIT-YYYY-MM-DD.md** — deep state-of-the-repo audit
>   2. **ROADMAP-YYYY-MM.md** — monthly plan, broken into 4 weeks
>   3. **WEEK-WW.md** × 4 — week-by-week scope with daily granularity
>   4. **daily/YYYY-MM-DD.md** × ~20 — a fully self-contained AVTONOM prompt
>      for every working day in the month
>
> After bootstrap, each subsequent day the operator opens Claude Code,
> reads `docs/session-plans/daily/<today>.md`, and pastes it verbatim as
> the session-opening message. The daily prompt is engineered so that
> **AVTONOM asks no questions and requests no approvals** — every decision
> point has a pre-resolved default and a hard-stop fallback.
>
> The bootstrap itself runs in AVTONOM mode with the same discipline as
> `avtonom-next-session.md`: no `git push`, no spine-file edits, all
> defaults pre-resolved.

---

```
AVTONOM: AX•CMS · MONTH-BOOTSTRAP — deep audit + 4-week roadmap + per-week
detail + per-day self-contained AVTONOM prompts. Я недоступен на ~30 дней.
Не спрашивай разрешений. Дефолты ниже — финальные.

═══════════════════════════════════════════════════════════════════════════
PRE-FLIGHT (обязательно, до Phase A)
═══════════════════════════════════════════════════════════════════════════

PF1.0. Прочитать docs/session-plans/RETRO-2026-06.md (full); §7
       (Next-month proposals) is the SEED for B1 goals — do not
       re-derive from AUDIT alone, blend with RETRO. Also read
       docs/governance/master-plan-diffs/MPD-001-commerce-crm-pivot.md
       — commerce + CRM threads land at M2 W6 D1 per MPD-001 §3
       (additive domain restructure, no rewrite).

PF1. Прочитать целиком:
       - ENTITY.md
       - CLAUDE.md
       - последний SESSION_LOG.md в корне NaSV2/
       - последний docs/session-plans/avtonom-*.md (предыдущий promt)
       - memory/MEMORY.md (если есть)
       - apps/web/public/platform-blueprint.html (если есть) — там
         «План → Статус» от пользователя
PF2. Прочитать `git log --oneline -50` и `git status --short`.
PF3. Запустить P0 verification gate (как в обычной AVTONOM):
       V1 cargo check --workspace --all-targets → docs/session-logs/...
       V2 cargo fmt --all
       V3 cargo clippy --workspace --all-targets -- -D warnings
       V4 cargo test --workspace --lib --no-fail-fast
     Если что-то красное — ЧИНИ (non-spine), макс 5 итераций. Если выйти
     не получается — записать в AUDIT.md как «P0-blocker», но Phase A
     всё равно начать.
PF4. Текущая дата = `MONTH_START`. Каждый «working day» = пн-пт. Считать
     ~20 рабочих дней вперёд. Список дней зафиксировать в AUDIT.md.

═══════════════════════════════════════════════════════════════════════════
PHASE A · DEEP AUDIT  → docs/session-plans/AUDIT-{MONTH_START}.md
═══════════════════════════════════════════════════════════════════════════

Структура AUDIT (markdown, без лимита по объёму — лучше длинно и точно):

A1. **Stack inventory** — пройти crates/* и apps/*. Для каждого crate:
      - назначение в одну строку
      - implementation status: stub | partial | functional | production
      - LOC реального кода (не комментариев), кол-во unit-тестов,
        кол-во #[ignore]-тестов, наличие benches
      - явные TODO / FIXME / unimplemented!() / todo!()
      - явные нарушения ENTITY.md §1-§10 (если видишь — фиксируй)

A2. **Migration inventory** — `ls migrations/`. Для каждой:
      - что делает (одна строка)
      - expand-only или contraction
      - есть ли соответствующий docs/rollback/ROLLBACK-*.md

A3. **Planning trail audit** — пройти docs/rfc/, docs/adr/, docs/plans/,
    docs/validations/. Построить таблицу:
      | RFC-NNN | ADR-NNN | PLAN-NNN | VAL-NNN | Implemented? | Notes |
    Пометить «orphan» документы (RFC без ADR, PLAN без VAL и т.д.)
    и «orphan» код (фичу без RFC/ADR).

A4. **xtask gate audit** — для каждой команды в xtask/src/main.rs Cmd:
    stub vs real-impl. Перечислить, какие гейты ещё нужны (capability-
    coverage, magic-check, check-planning-refs, extension-audit,
    alloc-budget, query-budget, pgo-build, bolt-optimize).

A5. **CI audit** — `.github/workflows/nasv2-*.yml`. Список jobs, какие
    cargo команды дёргают, какие гейты ещё не в CI.

A6. **Observability audit** — что реально есть в crates/runtime/src/
    observability.rs, и каких слоёв из ENTITY §4 не хватает (Pyroscope,
    OTLP exporter, метрики per-category).

A7. **Security/threat surface** — пройти docs/security/, проверить
    наличие ammonia sanitize-on-write (§3.6), capability hash в cache key
    (§3.9.1), RLS coverage (§3.3 + §15). Список missing controls.

A8. **Performance baseline** — есть ли хоть один зафиксированный
    criterion baseline в docs/perf/? Если нет — пометить «no baseline».

A9. **Open recommendations** — собрать ВСЕ «Recommendations for human
    review» из последних 3 SESSION_LOG.md в один пронумерованный список
    с пометкой «closed/open» (cross-reference с реальным состоянием
    кода). Open = действительно нужно сделать в этом месяце.

A10. **Risk register** — топ-10 рисков, отсортировано по `вероятность ×
     импакт`. Источник: §8 (engineering rules), §9 (forbidden
     architecture), §22 (mode discipline), плюс A1-A8 выше.

A11. **Stack of unfinished sessions** — если в SESSION_LOG.md`Skipped/
     Blocked` остались items без follow-up даты — поднять их сюда.

A12. **Capacity model** — ~20 working days × 8 productive hours = ~160
     часов. Из них: 30% на P0 verification (gate per day), 50% на
     feature work, 15% на refactor/debt, 5% на docs. Привести
     числами для последующей недельной нарезки.

Commit: `chore(ax/audit): deep audit YYYY-MM-DD` — один файл AUDIT-*.md.

═══════════════════════════════════════════════════════════════════════════
PHASE B · MONTHLY ROADMAP  → docs/session-plans/ROADMAP-{YYYY-MM}.md
═══════════════════════════════════════════════════════════════════════════

Цель Phase B — превратить AUDIT в исполнимый план на 4 недели.

B1. **Goals (3-5 штук)** — каждая цель ссылается на §ENTITY и на
    конкретный gap из AUDIT A1-A10. Формат:
      G1 — <цель в одну строку>
        Why: <link to §ENTITY + AUDIT item>
        Success criterion: <измеримо: бенч/тест/CI gate green>
        Estimated days: <N>

B2. **Anti-goals** — что в этом месяце НЕ делаем (явный список, чтобы
    avtonom-сессии в течение месяца не ушли в сторону).

B3. **Weekly breakdown table:**
      | Week | Dates | Theme | Goals covered | Daily prompt count |
      |---|---|---|---|---|
      | W1  | M..F  | ...   | G1, G2        | 5 |
      | W2  | M..F  | ...   | G2, G3        | 5 |
      | W3  | M..F  | ...   | G3, G4        | 5 |
      | W4  | M..F  | ...   | G4, G5, slack | 5 |

B4. **Dependency graph** — ASCII-DAG между G1..G5, чтоб видно было,
    какие цели блокируют другие.

B5. **Slack budget** — оставить 10-15% дней пустыми / «catch-up day»
    для overrun.

B6. **Exit criteria for month** — что должно быть истинно к концу
    месяца, чтобы цели засчитать выполненными.

Commit: `docs(ax/roadmap): monthly plan YYYY-MM` — один файл
ROADMAP-YYYY-MM.md.

═══════════════════════════════════════════════════════════════════════════
PHASE C · WEEKLY DETAIL  → docs/session-plans/WEEK-{NN}.md × 4
═══════════════════════════════════════════════════════════════════════════

Для каждой недели создать WEEK-WW.md:

C1. **Header:** week number, dates, theme (из ROADMAP B3).

C2. **Daily slots** — таблица:
      | Date | Day | Daily prompt file | Scope summary | Est hours |

C3. **Dependencies satisfied entering this week** — что должно быть
    готово к понедельнику этой недели.

C4. **Definition of done for this week** — checklist.

C5. **Carry-over policy** — если день N не уложился, как переносится
    в день N+1 (по умолчанию: добавить в начало следующего daily
    prompt секцию `## CARRY-OVER from <date>`).

Commit: `docs(ax/week-NN): plan for W<N> YYYY-MM-DD..YYYY-MM-DD`.

═══════════════════════════════════════════════════════════════════════════
PHASE D · DAILY PROMPTS  → docs/session-plans/daily/{YYYY-MM-DD}.md × ~20
═══════════════════════════════════════════════════════════════════════════

Это главный артефакт. Каждый daily prompt — полностью самодостаточный
AVTONOM-промт, который оператор вставит как opening message и который
НЕ задаст ни одного вопроса.

Шаблон (генерировать строго по этой структуре):

```
# Daily AVTONOM — YYYY-MM-DD (W{NN} D{1..5})

> Self-contained. Paste verbatim as opening message in Claude Code at
> F:\Users\a\Documents\_DEV\Tran\ES\barbie\AX\NaSV2.

AVTONOM: AX•CMS · <тема дня в одну строку>.
Дефолты ниже — финальные. Вопросов не задавать. Не пушить.

═════════════════════════════════════════════════════════════════════
SCOPE (приоритет сверху вниз — пропускай на блоке, не выходи за рамки)
═════════════════════════════════════════════════════════════════════

P0 · Verification gate (как обычно — V1..V4 → docs/session-logs/avtonom-
     <date>-cargo-*.log; 5 итераций max; если красное и нечинибельное →
     SKIP остальных фаз, full report в SESSION_LOG, HARD STOP).

P1..PN · <конкретные шаги дня, сгенерированные из WEEK + ROADMAP>
  Для каждого: что создать/изменить, какие тесты, какие clippy-allow
  ожидаются, где spine-edit АВТОРИЗОВАН.

P{N+1} · Final — SESSION_LOG.md (overwrite) + git commit per phase +
        trailer `AI-Assisted: AX-ARCHITECT (Claude Opus 4.7)`.

═════════════════════════════════════════════════════════════════════
PRE-RESOLVED DEFAULTS (унаследовано от avtonom-next-session.md, плюс
дневные)
═════════════════════════════════════════════════════════════════════

  • Версии библиотек: workspace Cargo.toml — read-only. Новые deps в
    crates/<X>/Cargo.toml допускаются (non-spine).
  • SQL: sqlx::query (runtime) когда схема не гарантированно есть при
    cargo check; sqlx::query! когда есть.
  • Naming: snake_case modules, PascalCase types,
    test_<func>_<scenario>. #[ignore = "needs Docker"].
  • Git: sequential commits, без rebase/amend. pre-commit hook
    fail → SKIP, лог в SESSION_LOG, дальше.
  • PowerShell + bash: для git/cargo — bash (избежать CRLF в HEREDOC).

═════════════════════════════════════════════════════════════════════
HARD STOPS (записать в SESSION_LOG, остановиться, НЕ коммитить)
═════════════════════════════════════════════════════════════════════

  1. Spine-touch без явной авторизации выше.
  2. cargo check > 5 итераций красное.
  3. cargo update fail > 2 крейтов подряд.
  4. Disk full / OOM.
  5. Любая операция требующая internet beyond cargo registry (gh,
     curl, новый docker pull) — SKIP.

═════════════════════════════════════════════════════════════════════
ЗАПРЕЩЕНО АБСОЛЮТНО
═════════════════════════════════════════════════════════════════════

  • git push (никогда, ни branch, ни tags, ни force).
  • Изменение ENTITY.md, CLAUDE.md, Cargo.toml (workspace),
    docker-compose.dev.yml, .env.example, clippy.toml, rustfmt.toml,
    rust-toolchain.toml, deny.toml.
  • Удаление файлов вне списка scope выше.
  • cargo update --workspace.
  • Создание PR через gh.
  • Полные benchmark runs (только --no-run).
  • Запуск xtask bench-runner (создаёт baseline на этой машине —
    operator делает на тихой машине).
  • Refactoring вне scope.

═════════════════════════════════════════════════════════════════════
SESSION_LOG.md формат
═════════════════════════════════════════════════════════════════════

# SESSION_LOG — AVTONOM <date> HH:MM (W{NN} D{N})

## Outcome — one line per phase
## Plan (detailed status)
## AI-Defaults applied (table Decision | Choice | Reason)
## Skipped / Blocked (table Item | Reason | Suggested follow-up)
## Commits made (local, not pushed) — table Phase | SHA | Title
## Recommendations for human review (numbered)
## Working tree at end of session (git status --short)
## Time budget (Started / Ended / Wall / Phases / Hard stops / SKIPs)
## CARRY-OVER for tomorrow (если что-то не успел — в начало
   daily/<next-day>.md как ## CARRY-OVER block)
```

Конец шаблона.

При генерации каждого daily prompt из WEEK + ROADMAP:
  - Scope конкретных P1..PN наполни на основе AUDIT.md gap-list +
    weekly slot.
  - Если день — W4 D5 (последний рабочий день месяца) — добавить P{last}
    «Monthly retrospective» с генерацией docs/session-plans/RETRO-
    {YYYY-MM}.md.
  - Если день — first Monday after major scope change — добавить P0.5
    «micro-audit» (re-read SESSION_LOG за прошлую неделю).
  - Carry-over slot: каждый daily prompt начинается с пустого
    `## CARRY-OVER from yesterday: (none)` — последующая AVTONOM
    редактирует это место, если перенос произошёл.

Commit одним пакетом: `docs(ax/daily): daily prompts for YYYY-MM`
— все ~20 файлов в одном коммите (они логически связаны).

═══════════════════════════════════════════════════════════════════════════
PHASE E · SELF-PACING HARNESS  → docs/session-plans/HOW-TO-RUN.md
═══════════════════════════════════════════════════════════════════════════

Создать docs/session-plans/HOW-TO-RUN.md — инструкция для оператора
(или для будущего ИИ):

E1. Каждый рабочий день: открыть Claude Code в NaSV2/, прочитать
    `docs/session-plans/daily/$(date +%F).md`, вставить как opening
    message, нажать enter, не вмешиваться.

E2. Если день пропущен (выходной / болезнь): следующий рабочий день
    AVTONOM в начале SCOPE добавляет секцию «PRIOR-DAY ROLLOVER:
    <prev-date>» — забирает carry-over оттуда.

E3. Если AVTONOM написал HARD STOP — оператор читает причину в
    SESSION_LOG.md, чинит блокер, и следующий день продолжает.

E4. В конце месяца — RETRO-{YYYY-MM}.md существует автоматически
    (Phase D last-day handler). Из retro генерируется следующий
    bootstrap (commit его как
    `docs/session-plans/avtonom-month-bootstrap-{YYYY-MM+1}.md`).

E5. Опционально: если оператор хочет полностью hands-off — оборачивает
    bootstrap в `/loop 24h /run docs/session-plans/daily/$(date +%F).md`
    (или эквивалент через скрипт). Этого bootstrap НЕ делает сам — это
    выбор оператора.

Commit: `docs(ax/harness): daily-prompt run instructions`.

═══════════════════════════════════════════════════════════════════════════
PHASE F · FINAL REPORT
═══════════════════════════════════════════════════════════════════════════

F1. SESSION_LOG.md (overwrite) — стандартная структура AVTONOM + extra:
      ## Generated artifacts
      | Type | Path | Count |
      ## Next action for operator
      «Открой docs/session-plans/daily/{MONTH_START}.md и вставь его
       как первое сообщение в новой сессии. Дальше — каждый день
       очередной daily prompt. RETRO будет создан автоматически в
       последний рабочий день месяца.»

F2. Коммиты структурой:
      1. Phase A audit       — 1 commit
      2. Phase B roadmap     — 1 commit
      3. Phase C weeks       — 1 commit (все 4 недели вместе)
      4. Phase D daily prompts — 1 commit (все ~20 dailies вместе)
      5. Phase E harness     — 1 commit
      6. Phase F SESSION_LOG — 1 commit
    Trailer `AI-Assisted: AX-ARCHITECT (Claude Opus 4.7)`.

F3. git status --short → в SESSION_LOG секцию «Working tree at end of
    session».

F4. git push — **никогда**.

═══════════════════════════════════════════════════════════════════════════
PRE-RESOLVED DEFAULTS (применяются ко всему bootstrap)
═══════════════════════════════════════════════════════════════════════════

  • Если MONTH_START — выходной (сб/вс), первый рабочий день =
    следующий пн. Не сдвигать диапазон месяца — он привязан к
    календарю, а не к 30-дневному окну.
  • Праздничные дни: НЕ исключать (ИИ не знает, праздник ли). Если
    оператор не появился — carry-over автоматом перенесёт.
  • Если AUDIT обнаружит, что какая-то цель из B1 уже выполнена
    (commit в git log) — пометь её как «✅ already done» и НЕ занимай
    её слотом в WEEK plan.
  • Если AUDIT обнаружит блокирующий риск (например, миграция не
    применима, поломанный CI) — назначь первую неделю на устранение
    блокера, остальные цели сдвинь.
  • Никаких новых тем, выходящих за §3 target stack ENTITY. Если
    в AUDIT появилась идея «давайте перейдём на ...» — записать в
    AUDIT раздел «Ideas for next month», НЕ в roadmap.
  • Все имена файлов даты — ISO (YYYY-MM-DD), без timezone.
  • Все daily prompts — на русском основном языке с английскими
    техническими терминами (как этот текст).

═══════════════════════════════════════════════════════════════════════════
HARD STOPS (для bootstrap)
═══════════════════════════════════════════════════════════════════════════

  1. Phase A не завершён за > 90 минут wall time → сохрани частичный
     AUDIT.md, переходи к Phase B на основе того что есть, в
     SESSION_LOG помечай как «AUDIT partial».
  2. Phase D не успел сгенерировать все daily prompts → сгенерируй
     минимум первые 5 (вся W1), остальные пометь stub-файлами с
     инструкцией «next bootstrap fills me in»; в SESSION_LOG —
     «daily prompts partial: 5/20».
  3. Spine-touch без авторизации в этом промте → НЕ делать; SKIP
     соответствующую часть, лог.
  4. Disk / OOM → STOP.

═══════════════════════════════════════════════════════════════════════════
ЗАПРЕЩЕНО АБСОЛЮТНО (для bootstrap)
═══════════════════════════════════════════════════════════════════════════

  • git push, gh PR, изменение spine-files, удаление чего-либо вне
    docs/session-plans/, refactor продакшен-кода. Bootstrap — это
    docs-only сессия (плюс P0 verification, который read-only).
  • Никаких новых cargo deps в этой сессии (если AUDIT обнаружит
    необходимость — это задача для будущих daily prompts).
  • Не изменять SESSION_LOG.md прошлых сессий (только текущий
    перезаписать).

═══════════════════════════════════════════════════════════════════════════
ОЖИДАЕМЫЙ РЕЗУЛЬТАТ (для верификации)
═══════════════════════════════════════════════════════════════════════════

После завершения bootstrap должны существовать:

  docs/session-plans/AUDIT-{MONTH_START}.md
  docs/session-plans/ROADMAP-{YYYY-MM}.md
  docs/session-plans/WEEK-01.md
  docs/session-plans/WEEK-02.md
  docs/session-plans/WEEK-03.md
  docs/session-plans/WEEK-04.md
  docs/session-plans/daily/YYYY-MM-DD.md    (× ~20)
  docs/session-plans/HOW-TO-RUN.md

И 6 коммитов на main (не push).

И SESSION_LOG.md в корне NaSV2/ с полным отчётом.

Дальше оператор каждое утро открывает Claude Code, вставляет содержимое
очередного daily prompt — и AVTONOM работает без вопросов.
```

---

## Как пользоваться

1. **Однократно:** открой Claude Code в
   `F:\Users\a\Documents\_DEV\Tran\ES\barbie\AX\NaSV2`, скопируй
   блок выше (от `AVTONOM:` до закрывающей ```), вставь как первое
   сообщение. Жди ~30-60 минут (Phase A + D занимают больше всего).
2. **Каждый рабочий день:** открой Claude Code в той же папке,
   прочитай `docs/session-plans/daily/$(date +%F).md`, вставь как
   первое сообщение. Не вмешивайся.
3. **В конце месяца:** AVTONOM в последний рабочий день месяца
   создаст `docs/session-plans/RETRO-{YYYY-MM}.md` и предложит
   следующий bootstrap. Запусти его — и следующий месяц поедет
   так же.

---

## Что bootstrap **никогда** не сделает сам

- `git push` (только оператор)
- изменение ENTITY.md / CLAUDE.md / workspace Cargo.toml / любого
  spine-файла (§12 ENTITY)
- создание PR на GitHub
- запуск долгих benchmark runs
- удаление существующих файлов вне `docs/session-plans/`

---

## Чек-лист перед запуском

- [ ] Docker НЕ обязан быть запущен (P0 V4 не трогает `#[ignore]`)
- [ ] `cargo` registry доступен (один `cargo search tokio` подтвердит)
- [ ] Питание ноутбука: power plan → High performance, Sleep = Never
- [ ] Disk free ≥ 5 GB (Phase A читает много файлов; Phase D пишет много)
- [ ] Текущие локальные коммиты — на той ветке, которую ты считаешь
      главной (по умолчанию `main`)
