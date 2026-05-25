# AVTONOM next-session prompt

> Готовый промт для следующей AVTONOM-сессии AX•CMS (NaSV2).
> Открой Claude Code в `F:\Users\a\Documents\_DEV\Tran\ES\barbie\AX\NaSV2`
> и вставь блок ниже целиком как первое сообщение.
>
> **Контекст:** продолжение работы после AVTONOM 2026-05-25 (коммиты
> `d149811..3325dc5`, фазы P0–P7). Этот промт двигает дальше — фазы 8–12:
> TaskSupervisor, pgmq adapter, первый осмысленный bench, закрытие
> рекомендаций из прошлого SESSION_LOG, real-impl двух xtask gate'ов.

---

```
AVTONOM: AX•CMS · продолжение AX-ARCHITECT refoundation, фазы 8-12.
Я недоступен до утра. Не спрашивай разрешений. Все defaults ниже — финальные.

═══════════════════════════════════════════════════════════════════════════
SCOPE (приоритет сверху вниз — выполняй последовательно, пропускай на блоке)
═══════════════════════════════════════════════════════════════════════════

P0 · Verification gate (как в прошлый раз — обязательно)
  V1. cargo check --workspace 2>&1 → docs/session-logs/avtonom-YYYYMMDD-cargo-check.log
  V2. cargo fmt --all (применить)
  V3. cargo clippy --workspace --all-targets -- -D warnings 2>&1 → лог.
      Если warnings/errors → исправь (non-spine файлы, спайн только если
      явно авторизован ниже). Лимит: 5 итераций на каждый из V1/V3.
  V4. cargo test --workspace --lib --no-fail-fast 2>&1 → лог.
      Если что-то падает в lib unit-тестах — это P0, чини. #[ignore] не
      запускаются, проблем с Docker не будет.

P1 · TaskSupervisor (crates/runtime/src/supervisor.rs) — ENTITY §4.8
  S1. Реализовать TaskSupervisor с полями:
        - category: TaskCategory enum { Http, Queue, Image, Report,
          Email, SearchIndex }
        - cancel: CancellationToken (tokio_util::sync)
        - handle: TaskTracker  (tokio_util::task::TaskTracker)
      Методы:
        - pub fn new(category: TaskCategory) -> Self
        - pub fn spawn<F>(&self, name: &'static str, fut: F) -> TaskHandle
          где F: Future<Output = ()> + Send + 'static
        - pub fn cancel_token(&self) -> CancellationToken (для проброса в задачи)
        - pub async fn drain(self, timeout: Duration) -> Result<(), DrainError>
      Внутри spawn — обёртка в tracing::Span (category + name + UUID
      task_id), обёртка в tracker.spawn(...). Никакого голого
      tokio::spawn нигде в проекте.
  S2. Добавить TaskHandle (тонкая обёртка над JoinHandle с tracing
      контекстом).
  S3. Unit-тесты (не #[ignore]):
        - test_spawn_and_drain_completes
        - test_drain_times_out_on_stuck_task → DrainError::Timeout
        - test_cancel_token_propagates
  S4. crates/runtime/src/lib.rs — pub mod supervisor; pub use ...
  S5. apps/server/src/main.rs (spine — mini-edit АВТОРИЗОВАН):
        создать один TaskSupervisor для category=Http при старте,
        прокинуть его в AppState. Пока ничего им не спавним — этот шаг
        только для wiring.

P2 · pgmq queue adapter (crates/infrastructure/src/queue/) — ENTITY §3.8
  Q1. Если crates/infrastructure/src/queue/ не существует — создай.
      mod.rs + pgmq.rs.
  Q2. PgmqQueue::new(worker_pool: PgPool) — конструктор.
  Q3. Методы (через sqlx::query!  напрямую против pgmq schema):
        - send(queue: &str, payload: &serde_json::Value) -> Result<i64>
        - read(queue: &str, vt_seconds: i32, qty: i32) -> Result<Vec<PgmqMessage>>
        - delete(queue: &str, msg_id: i64) -> Result<bool>
        - archive(queue: &str, msg_id: i64) -> Result<bool>
      Все запросы — через `SELECT pgmq.send(...)` etc., без N+1.
  Q4. Создать migration migrations/0XXX_pgmq_bootstrap.sql:
        CREATE EXTENSION IF NOT EXISTS pgmq;
        SELECT pgmq.create('ax_image_jobs');
        SELECT pgmq.create('ax_email_outbox');
        SELECT pgmq.create('ax_search_reindex');
      Номер 0XXX — следующий после последней existing миграции
      (посмотри ls migrations/).
  Q5. Unit-тесты MockQueue trait (если уже есть application/ports/queue.rs —
      проверь, не дублируй; иначе создай trait Queue в application/ports/).
  Q6. Integration-тест #[ignore = "needs Docker + pgmq"]:
        crates/infrastructure/tests/pgmq_integration.rs
        - send → read → delete round-trip против testcontainers postgres
          с pgmq extension (используй postgres:16 + установить pgmq из
          packagecloud, ИЛИ ghcr.io/tembo-io/pgmq image если есть готовый).
        Если pgmq extension не доступен в чистом postgres контейнере и
        требует кастомного образа которого нет в registry — log SKIP в
        SESSION_LOG и оставь только compile-tests.

P3 · Первый осмысленный criterion bench
  B1. crates/pool-validator/benches/pool_mode_check.rs:
        bench detect_pool_mode latency против локального postgres (если
        DATABASE_URL_DIRECT set в env — используй; иначе MARK
        bench как ignored через bencher.iter без подключения).
        Минимум: bench парсинга PoolMode::from_str для 4 вариантов.
  B2. crates/common/benches/ — если есть TenantId/RequestId/etc, bench
      их Default/Copy/serde. Если crates/common пустой — пропусти.
  B3. Cargo.toml каждого затронутого crate — добавить
        [[bench]]
        name = "..."
        harness = false
      и criterion в [dev-dependencies] (workspace = true).
  B4. cargo bench --workspace --no-run — должен скомпилироваться без
      ошибок. ПОЛНЫЙ запуск bench НЕ делай (долго).
  B5. cargo run -p xtask -- bench-runner НЕ запускай (создаст baseline
      на твоих локальных шумных числах — пусть пользователь сам делает
      первый baseline на спокойной машине).

P4 · Закрыть рекомендации из прошлого SESSION_LOG
  R1. crates/pool-validator/tests/pool_mode_integration.rs — добавить
      четвёртый тест:
        test_concurrent_load_isolation
        #[ignore = "needs Docker"]
        Запускает 50 параллельных tokio задач каждая делает
        ensure_transaction_mode — проверяет что не блочат друг друга
        (acquire_timeout ≤ 2s).
  R2. ops/pgbouncer/Dockerfile (НОВЫЙ) — production-ready образ:
        FROM edoburu/pgbouncer:1.23.1
        COPY pgbouncer.ini /etc/pgbouncer/pgbouncer.ini
        COPY databases.ini /etc/pgbouncer/databases.ini
        COPY userlist.txt /etc/pgbouncer/userlist.txt
        # auth_type для prod — комментарий что меняется на scram-sha-256
        # через env override в production-compose
      Документировать в docs/plans/PLAN-003 что это «бейк» для prod
      override.
  R3. docs/validations/VAL-004-task-supervisor.md — VAL для P1.
  R4. docs/plans/PLAN-004-pgmq-bootstrap.md — PLAN для P2.

P5 · xtask: capability-coverage real impl (если crates/presentation уже
     имеет какие-то handler'ы — иначе пропусти, оставь stub)
  C1. ls crates/presentation/src — если есть handlers/router, реализуй
      xtask/src/commands/capability_coverage.rs:
        regex-scan на pub async fn .*Handler|.*handler — все совпадения
        должны иметь .require_capability(...) либо явный аннотацию
        #[no_capability_required] (атрибут можно сделать заглушкой —
        просто комментарий). Список нарушителей → stderr + exit 1.
      Wire через xtask/src/main.rs (spine — mini-edit АВТОРИЗОВАН для C1,
      аналогично X1 в прошлой сессии).
  C2. Если crates/presentation/src пустой / без handler'ов — log SKIP
      в SESSION_LOG, переходи дальше.

P6 · Architecture-check real impl
  A1. xtask/src/commands/architecture_check.rs:
        - Парсит Cargo.toml каждого crate в workspace через cargo_metadata
          крейт (добавь в xtask/Cargo.toml).
        - Применяет правила ENTITY §2.6:
            domain/ зависит только от: serde, uuid, chrono, garde, thiserror
            domain/ НЕ зависит от: tokio, sqlx, axum, reqwest, sentry, tracing
            presentation/ НЕ зависит от infrastructure/ напрямую
            infrastructure/ НЕ exports sqlx::Row / sqlx::Pool — это уже сложно
              проверить без полного AST, поэтому проверь только deps уровень.
        - Нарушители → stderr + exit 1.
  A2. Wire в xtask/src/main.rs (spine — mini-edit АВТОРИЗОВАН для A2).
  A3. ENTITY §2.6 хочет ещё magic-check, check-planning-refs etc — НЕ
      реализуй их в этой сессии, оставь stub'ами (как в прошлой
      бенчмарк PGO/BOLT).

P7 · Final
  F1. SESSION_LOG.md в корне NaSV2/ — итоговый отчёт. ПЕРЕЗАПИШИ
      целиком, не append'и к старому (старый SESSION_LOG из прошлой
      AVTONOM теперь в git history, что хочется сохранить — уже в
      коммите 3325dc5).
  F2. Один коммит на P-X, trailer: AI-Assisted: AX-ARCHITECT (Claude Opus 4.7).
  F3. После всех коммитов — git status --short (вставить вывод в
      SESSION_LOG.md секцию "Working tree at end of session"). Должно
      быть пусто или только заранее существовавшие parent-repo M-файлы.

═══════════════════════════════════════════════════════════════════════════
PRE-RESOLVED DEFAULTS (не спрашивай — выполняй так)
═══════════════════════════════════════════════════════════════════════════

  • Версии библиотек: workspace Cargo.toml как single source of truth,
    ничего не bump'ай. Если зависимости (cargo_metadata, etc) нужны и
    их нет — добавляй с conservative версией (LATEST_KNOWN_STABLE_MINUS_ONE
    если можешь определить, иначе любая, лишь бы cargo check прошёл).
  • clippy allow attributes: НЕ ленись писать `// reason: ...` рядом
    с #[allow(...)]. Один-два слова достаточно.
  • Naming: snake_case модули, PascalCase типы, имена тестов
    test_<функция>_<сценарий>. #[ignore = "needs Docker"] / "needs pgmq".
  • Git: sequential commits, без rebase, без amend. Если pre-commit
    hook падает — попытайся починить корневую причину 1 раз; если не
    выходит — НЕ используй --no-verify, оставь файлы staged, log в
    SESSION_LOG, переходи к следующей фазе.
  • Если parent git заняли параллельные коммиты (как было в прошлый
    раз с auth migration) — игнорируй их, работай только с NaSV2/.
  • Если cargo загрузка крейта падает (timeout / 404) — попробуй
    cargo update только для этого крейта (cargo update -p <crate>);
    если не помогает — закомментируй фичу, log SKIP, переходи дальше.
  • PowerShell здесь, но Bash работает через harness — используй bash
    для git/cargo чтоб не ловить CRLF-сюрпризы в HEREDOC коммитах.

═══════════════════════════════════════════════════════════════════════════
HARD STOPS (только тут останавливайся и пиши в SESSION_LOG, без коммита)
═══════════════════════════════════════════════════════════════════════════

  1. Spine-touch БЕЗ авторизации выше (см. список «авторизовано» в
     P1 S5, P5 C1, P6 A2; всё остальное в spine — SKIP).
  2. cargo check падает > 5 итераций — SKIP фазы, stderr в SESSION_LOG.
  3. cargo update fail на >2 крейтах подряд (registry проблема) — STOP,
     дождись утра.
  4. Disk full / OOM — STOP.
  5. Любая операция требующая internet beyond cargo registry (gh CLI,
     curl, docker pull нового образа) — SKIP.

═══════════════════════════════════════════════════════════════════════════
ЗАПРЕЩЕНО АБСОЛЮТНО
═══════════════════════════════════════════════════════════════════════════

  • git push (ни branch, ни tags, ни force).
  • Изменение ENTITY.md, CLAUDE.md, Cargo.toml (workspace),
    docker-compose.dev.yml, .env.example, clippy.toml,
    rustfmt.toml, rust-toolchain.toml, deny.toml.
  • Удаление файлов которых нет в задачах выше (там их нет — значит
    ничего не удаляй).
  • cargo update --workspace (bump всего lockfile разом).
  • Создание PR через gh.
  • Запуск долгих benchmark'ов целиком (cargo bench без --no-run).
  • Запуск xtask bench-runner (создаст baseline на твоих числах).
  • Refactoring beyond scope. Только P0..P7.
  • Любые правки в parent ax/ (всё что вне NaSV2/ кроме
    .github/workflows/nasv2-*.yml).

═══════════════════════════════════════════════════════════════════════════
SESSION_LOG.md формат — как в прошлый раз
═══════════════════════════════════════════════════════════════════════════

# SESSION_LOG — AVTONOM YYYY-MM-DD HH:MM

## Outcome — one line per phase
## Plan (detailed status)
## AI-Defaults applied (таблица Decision | Choice | Reason)
## Skipped / Blocked (таблица Item | Reason | Suggested follow-up)
## Commits made (local, not pushed) — таблица Phase | SHA | Title
## Recommendations for human review (нумерованный список)
## Working tree at end of session (вывод git status --short)
## Time budget (Started/Ended/Wall time/Phases/Hard stops/SKIPs)
```

---

## Перед уходом — короткий чек-лист

- [ ] Docker НЕ обязан быть запущен — `#[ignore]`-тесты не выполняются
      автоматически в P0 V4 (он гоняет только `--lib`).
- [ ] `cargo` registry доступен (один `cargo search tokio` подтвердит).
- [ ] Текущие 8 коммитов (`d149811..3325dc5`) ещё не запушены —
      следующая сессия унаследует их и добавит сверху ещё 7-8 своих.
- [ ] Питание ноутбука: исключить сон. Power plan → High performance,
      Sleep = Never пока висит сессия.
- [ ] Claude Code в той же папке: `F:\Users\a\Documents\_DEV\Tran\ES\barbie\AX\NaSV2`.
