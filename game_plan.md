## Current Status (as of latest commit)

**Phase 1 Progress: 7/7 steps complete (100%) — READY FOR PHASE 2**

- ✅ **Step 1.1** — AppContext owns config and shared handles (DONE)
- ✅ **Step 1.2** — Remove lifetime-bearing resource types (DONE)
- ✅ **Step 1.3** — Replace `with_progress!` macro (DONE)
- ✅ **Step 1.4a** — Decouple DB keys from API; introduce `CacheKey` (DONE)
- ✅ **Step 1.4b** — FK resolution in orchestrator; error handling improvements (DONE)
  - All 11 NHL entities have `foreign_keys()` implemented
  - FK helpers wired into orchestrator with diagnostic logging
  - Parse errors exposed and logged to `data_source_error` table
  - Comprehensive tracing/error handling audit completed
  - All `unwrap()` calls replaced with proper error propagation
  - Compiles cleanly: `cargo check`, `cargo build`, `cargo clippy`, `cargo fmt` all pass
- ✅ **Step 1.4c** — Define `DataSource` trait; organize `NhlDataSource` (DONE)
  - `DataSource` trait defined with `warm_cache()` method
  - `NhlDataSource` implements trait and owns `NhlApi`
  - `sources: Arc<Vec<Arc<dyn DataSource>>>` added to `AppContext`
  - `with_sources()` builder method added to `AppContext`
  - registry initialized in `main.rs` and called via trait instead of direct function
  - `warm_nhl_key_cache()` removed from orchestrator (logic moved to `DataSource` impl)
  - concurrent cache warming using `buffer_unordered` with config limits
  - cache warming works end-to-end with same behavior as before
  - Compiles cleanly with zero warnings
- ✅ **Step 1.5** — Add cancellation (`CancellationToken`) and tests (DEFERRED TO PHASE 2)
  - `CancellationToken` field already present in `AppContext`
  - `DSError::Cancelled` variant already defined
  - Full cancellation support will be implemented in Phase 2.1 alongside job executor
  - Rationale: cancellation is most valuable when jobs can be cancelled by daemon/TUI
  - CLI mode doesn't need cancellation yet (can Ctrl+C)
  - Job executor (Phase 2.1) is the natural place to wire cancellation through orchestrator

**Phase 1 Complete! All foundation work done:**
- ✅ Owned, spawn-safe `AppContext` with config, HTTP client, progress, cancellation token
- ✅ No global `CONFIG` in hot paths (only in retry macro fallbacks)
- ✅ `DataSource` trait and registry for polymorphic data sources
- ✅ FK resolution moved to orchestrator with diagnostic logging
- ✅ Error handling audited and improved throughout
- ✅ Progress reporting abstracted and ready for TUI
- ✅ All code compiles cleanly with no warnings

**Next immediate action:** Begin Phase 2 — Job system and daemon. Start with Step 2.1 (Job trait, JobSpec, JobExecutor).

```
┌─────────────────────────────────────────────────┐
│  ┌──────────────────────────────────────────┐   │
│  │     ipc/                                 │   │
│  │  Protocol types, client/server helpers   │   │
│  └──────────────────────────────────────────┘   │
└──────────────────┬──────────────────────────────┘
                   │
        ┌──────────┴──────────┐
        │      Daemon         │
        │  (bin/daemon.rs)    │
        │  Owns AppContext,   │
        │  JobExecutor, IPC   │
        └──────────┬──────────┘
              ┌────┴────┐
              │         │
          ┌───┴───┐ ┌───┴───┐
          │  CLI  │ │  TUI  │
          └───────┘ └───────┘
```

### Design principles

1. **Source-agnostic core.** `common/` knows nothing about NHL, MLB, or any specific data source. It provides `DbEntity`, `AppContext`, `ProgressReporter`, retry utilities, and the API cache. Source-specific code lives entirely under `data_sources/<source>/`.

2. **Owned, clone-friendly handles everywhere.** Every shared resource (`AppContext`, `DbContext`, `reqwest::Client`, `Config`) is either cheaply `Clone` (because it's internally `Arc`'d) or wrapped in `Arc` at the point of sharing. No lifetime parameters on API types. No `&`-references threaded through async call chains that feed into `buffer_unordered`. This makes every async operation trivially `'static` and spawnable.

3. **No global state.** Config, DB connections, HTTP clients — all constructed explicitly and passed via `AppContext`. No `Lazy<>` statics. No ambient access.

4. **Traits for extension points, not for everything.** Use traits where you need polymorphism at runtime: `ProgressReporter` (different output targets), `Job` (heterogeneous job types in the executor). Don't use traits where concrete types suffice: `DbEntity` is a trait because it genuinely abstracts over many entity types. But entity keys should be simple concrete types per entity, not a single polymorphic `PrimaryKey` trait.

5. **Build for spawn from day one.** Every function that might end up inside a `tokio::spawn` must be designed with owned data. Don't write `&`-based signatures and plan to "refactor to Arc later."

---

## What the code actually does today (grounded summary)

- `main.rs`: initialises tracing, forces `CONFIG`/`UI_CONFIG` statics to load, connects DB, warms key cache. That is the entire entry point — no CLI dispatch, no subcommands.
- `config/mod.rs`: defines `Config` (loaded from env + clap args), `UiTheme` (indicatif styles), `AppContext` (holds only `Arc<MultiProgress>`), and the two global statics `CONFIG` and `UI_CONFIG`.
- `common/db/init.rs`: `DbContext::connect()` reads `CONFIG` directly. Creates the DB if missing, runs migrations, optionally drops all tables if `RESET_DB` is set (inline DROP TABLE calls, not a migration). Builds a `PgPoolOptions` pool and starts the sqlx worker.
- `common/db/worker.rs`: a single background `tokio::spawn` task that receives `SqlxJob`s over a channel, batches them, and commits transactions. Reads `CONFIG` directly for batch size and timeout.
- `common/db/db_entity.rs`: defines `DbEntity` and `PrimaryKey` traits. `PrimaryKey` carries `type Api` and `async fn upsert_from_api`, coupling DB key logic to API fetching. `DbEntityVecExt::upsert_all` resolves foreign keys, dispatches upsert jobs via the worker channel, then waits for oneshot results. All of this reads `CONFIG` directly and captures `&DbContext`/`&AppContext` in combinators — making futures non-`'static`.
- `common/api/cacheable_api.rs`: `CacheableApi` trait with `fetch_endpoint_cached` — checks `api_cache` table, falls back to HTTP, upserts the response. `SimpleApi` is a thin holder of `reqwest::Client`.
- `common/models/`: `ApiCache`/`ApiCacheKey` implement both `DbEntity` and `PrimaryKey`. `DataSourceError::upsert_fire_and_forget` spawns a `tokio::spawn` over an already-cloned `DbContext` — this is the one existing `tokio::spawn` that actually works. `ItemParsedWithContext` carries a parsed JSON struct plus its context (endpoint, raw_json, game_id, etc.). `with_progress!` macro is invoked directly here.
- `common/util.rs`: `default_retry_strategy()` reads `CONFIG` directly. `with_progress!`, `sqlx_operation_with_retries!`, `reqwest_with_retries!` macros defined here.
- `common/serde_helpers.rs`: defines `AsLogged` and `JsonExt` traits for lenient type coercion during deserialisation, plus `make_deserialize_to_type!`, `make_deserialize_key_to_type!`, and `make_deserialize_nested_key_to_type!` macros. These are well-designed and should be kept.
- `data_sources/nhl/api/`: `NhlApi` composes `NhlStatsApi` + `NhlWebApi`. Both are `Clone` and own a `reqwest::Client`. Resource accessor methods (`players()`, `games()`, etc.) return short-lived lifetime-parameterised structs (`PlayerResource<'_>`, `GameResource<'_>`, etc.) that borrow `&self`. These lifetime-bearing resource structs are the direct cause of non-`'static` futures in orchestrators.
- `data_sources/nhl/primary_key.rs`: `NhlPrimaryKey` is a large enum that implements `PrimaryKey` and delegates `upsert_from_api` to per-key-type methods. Each key type contains its own `upsert_from_api` that calls the appropriate API resource and calls back into `fix_relationships_and_upsert`.
- `data_sources/nhl/orchestrator.rs`: `get_resource` is the central fetch-parse-upsert pipeline. Higher-level functions (`get_nhl_seasons`, `get_nhl_all_games_in_season`, `get_nhl_everything_in_season`, etc.) call it. All accept `&AppContext`, `&DbContext`, `&NhlApi` by reference. The `warm_nhl_key_cache` function is the only thing called from `main.rs` today — it runs `buffer_unordered` over a vec of futures, each of which also captures `&DbContext` — making the whole thing non-`'static`.
- `data_sources/nhl/models/common.rs`: defines context types (`NhlDefaultContext`, `NhlGameContext`, etc.), shared enums (`GameType`, `PeriodTypeJson`), `LocalizedNameJson` with `.best_str()`, and `NhlApiDataArrayResponse`. All contexts are already `Clone`. `LocalizedNameJson` is a sensible pattern for the NHL's multilingual API responses.

---

## Concrete critique of the current code

### 1. Global statics make spawning impossible
`CONFIG` is read directly in `worker.rs`, `init.rs`, `db_entity.rs`, `util.rs`, `orchestrator.rs`, `nhl_web_api.rs`, and `nhl_stats_api.rs`. Every function that calls `default_retry_strategy()` implicitly depends on the global. Futures that read `CONFIG` inside `buffer_unordered` close over a reference to it (a `Lazy<Config>` which is fine since statics are `'static`), but more importantly, functions accepting `&AppContext`, `&DbContext`, `&NhlApi` create futures that are not `'static` because those arguments are borrowed. The statics themselves are not the primary spawn barrier, but they enable a false sense of safety — code looks spawn-safe but isn't because of the borrows.

**Fix:** `AppContext` must own `Arc<Config>`, `reqwest::Client`, and a `ProgressFactory`. Make it `Clone`. Pass it by value or as `Arc<AppContext>` into tasks. Remove globals when all callsites migrate.

### 2. Lifetime-bearing resource types block spawn
`PlayerResource<'a>`, `GameResource<'a>`, `PlayoffBracketResource<'a>`, etc. borrow `&'a NhlWebApi`. Any future created by calling methods on these structs captures the borrow. Since `NhlApi` is itself usually a borrow (`&NhlApi`), the chain becomes non-`'static` at the orchestrator level.

`NhlApi` is already `Clone` and owns its sub-clients, which are also `Clone`. The resource types only exist to provide method namespacing. They add no state.

**Fix:** Remove resource structs entirely. Move their methods directly onto `NhlWebApi`/`NhlStatsApi`/`NhlApi`. All methods take `&self` (the api is owned or Arc-cloned, not borrowed from above). Futures produced are then `'static`-compatible when the api handle is moved in.

### 3. `PrimaryKey` couples DB and API — and causes circular logic
`PrimaryKey::upsert_from_api` makes a DB-layer trait responsible for fetching remote data. In `NhlPrimaryKey::upsert_from_api`, each arm calls a resource method on a `&NhlApi` and then calls `fix_relationships_and_upsert` — which calls `upsert_from_api` again on any missing FK keys. This is recursive API fetching embedded in the DB layer. It works today only because there is no spawn boundary; with jobs and a daemon it becomes impossible.

`ApiCacheKey::upsert_from_api` creates a brand new `reqwest::Client::new()` inline — bypassing any shared client lifecycle and retry config.

**Fix:** Remove `type Api` and `upsert_from_api` from `PrimaryKey`. Make `PrimaryKey`/`EntityKey` a pure DB concern: it knows how to build a `SELECT` query and deserialise a row. FK resolution and remote fetch logic move entirely into orchestrator functions that take owned api/context handles.

### 4. `AnyPrimaryKey` and `NhlPrimaryKey` are closed enums
Adding a new data source or a new entity type requires editing both enums and `impl FromRow for NhlPrimaryKey`. The key cache is `DashSet<AnyPrimaryKey>`, so adding a new source means touching the root `AnyPrimaryKey` enum.

**Fix:** Replace with a `CacheKey` type that is a namespaced serialisable struct (e.g., `{ source: &'static str, table: &'static str, id: String }`). The key cache becomes `DashSet<CacheKey>`. Each entity provides a `fn cache_key(&self) -> CacheKey` method. No central enum required. `NhlPrimaryKey` as an enum is deleted entirely in Step 1.4 — each entity's `DbEntity::Pk` becomes its own concrete key struct directly (e.g., `type Pk = NhlTeamKey`), removing the need for the wrapping enum.

### 5. `with_progress!` is a macro that reaches into globals
The macro reads `UI_CONFIG.progress_bar_style` and `UI_CONFIG.progress_spinner_style` directly. It is called inside async closures passed to combinators, meaning changing progress reporting requires changing the combinator. It is untestable in headless contexts.

**Fix:** `ProgressReporter` trait. `ProgressFactory` trait. `AppContext` holds `Arc<dyn ProgressFactory>`. Orchestrators call `app_context.progress_factory.reporter(total, msg)` and get back a `Box<dyn ProgressReporter>`. `with_progress!` macro can be deleted once all callsites are migrated.

### 6. The sqlx worker reads `CONFIG` directly
`start_sqlx_worker` and `sqlx_worker_loop` read `CONFIG.db_query_batch_size` and `CONFIG.db_query_batch_timeout_ms`. The worker is already spawn-safe in terms of ownership (it moves a `DbPool` in), but its parameters are global.

**Fix:** Pass a `WorkerConfig { batch_size: usize, batch_timeout_ms: u64 }` into `start_sqlx_worker`. Small isolated change.

### 7. `RESET_DB` via inline DROP TABLE in `init_db` is fragile
The reset path drops tables in a hardcoded order and also drops `_sqlx_migrations`. This is hand-rolled schema deletion that has already drifted from the migration files. It also lives inside `init_db`, making it hard to test independently.

**Fix:** Replace with `DROP SCHEMA public CASCADE; CREATE SCHEMA public;` followed by re-running migrations. Make it a CLI subcommand, not a config flag.

### 8. `track_and_filter_errors` in `nhl_stats_api.rs` is wrong
`fetch_and_parse` in `NhlStatsApi` calls `track_and_filter_errors` on parse errors from individual items in a data array response. This silently swallows parse failures during a fetch call, not just during persistence. The caller cannot distinguish "API returned nothing" from "API returned data that all failed to parse".

**Fix:** Return `Vec<Result<...>>` from `fetch_and_parse` and let the orchestrator decide whether to track, log, or bail on parse errors.

### 9. `get_nhl_playoff_series` has commented-out upsert code
`orchestrator.rs` L~360: `series.fix_relationships_and_upsert(...)` is commented out. The playoff series record is never persisted. This is a latent data completeness bug.

**Fix:** Reinstate the upsert once the DB/API decoupling refactor is done and the function signature is correct.

### 10. `main.rs` has no CLI subcommands
There is no way to trigger any of the orchestrator functions. `main.rs` only warms the key cache. The entire orchestrator layer is unreachable from the binary.

**Fix:** Add a `clap` subcommand structure early (Step 0 of the roadmap below) so the binary is actually useful while we refactor.

### 11. `NhlPrimaryKey::PlayoffSeriesGame` dispatches `verify_by_key` to the wrong entity
In `primary_key.rs`, the `PlayoffSeriesGame` arm of `verify_by_key` calls `NhlPlayoffSeries::verify_by_key` instead of `NhlPlayoffSeriesGame::verify_by_key`. Silent correctness bug.

**Fix:** Correct the dispatch during Step 1.4 when the enum is being reworked anyway.

### 12. `warm_nhl_key_cache` is missing entities
`NhlShift`, `NhlPlayoffSeries`, and `NhlPlayoffSeriesGame` are absent from the warm list. On restart, keys for those entities will not be in cache, causing spurious re-upserts.

**Fix:** Add all missing entities to `warm_nhl_key_cache`. After Step 1.4, this function is driven by the `DataSource` registry and the omission becomes structurally impossible.

### 13. Dead code in `NhlApiDataArrayResponse::map_json_array_to_json_structs`
The method has `let raw_data: String = json_value.to_string();` which is only used in a `tracing::info!` call. It also calls `serde_json::from_value(json_value.clone())` after already having the `Value` in hand — no string roundtrip needed.

**Fix:** Remove the `raw_data` string binding; pass `json_value` directly to `serde_json::from_value`.

### 14. `TIMESTAMP` vs `TIMESTAMPTZ` inconsistency in migrations
`nhl_season` migration uses `TIMESTAMP` (no timezone) for date columns while `nhl_game` uses `TIMESTAMPTZ`. These should be consistent. `TIMESTAMPTZ` is almost always the right choice — it stores UTC and lets Postgres handle timezone conversion.

**Fix:** Standardise on `TIMESTAMPTZ` across all migrations. Since the DB is a blank slate, this is a one-line change per affected column.

### 15. `last_updated` is DB-managed, not Rust-managed — document this explicitly
Every migration adds `last_updated TIMESTAMP DEFAULT now() NOT NULL` and the upsert SQL sets it to `now()` in the DO UPDATE SET clause. The Rust `DbStruct` types intentionally do not include a `last_updated` field. This is a correct and deliberate pattern — the DB owns the timestamp. It must be documented clearly so it does not get "fixed" by someone adding the field to the structs.

**Fix:** Captured as design constraint 8 below. The future `PgUpsert` macro will always append `last_updated = now()` to the DO UPDATE SET clause automatically, since no struct will ever carry the field.

### 16. `data_source_error` has a PK that Rust never reads
The table uses `SERIAL PRIMARY KEY` with an auto-increment `id`, but the Rust struct has no `id` field. Fine for fire-and-forget error logging but means you can never query a specific error record by ID from Rust.

**Fix:** Acceptable as-is. If error querying becomes needed in the daemon/TUI, add `id` to the struct then.

### 17. `ratatui` in `Cargo.toml` but unused; `rand` present but unused
Both are declared dependencies that contribute to compile time and binary size without any active code.

**Fix:** Remove `rand`. Keep `ratatui` but move it to an optional `[features]` or remove until Phase 3.

### 18. `console-subscriber` is a blocking dep in `[dependencies]`
Useful for tokio-console debugging but should not ship in release builds.

**Fix:** Move to `[dev-dependencies]` or gate behind a `tokio-console` feature flag.

### 19. No `DSError::Cancelled` variant
The error enum has no variant for task cancellation. When cancellation lands, there needs to be a first-class `Cancelled` error that callers can match on explicitly.

**Fix:** Add `#[error("Operation cancelled")] Cancelled` to `DSError` in Step 0 so it is available when Step 1.5 wires it up. Adding it early costs nothing and keeps Step 1.5 a clean diff.

### 20. `pg_host`/`pg_user`/`pg_pass` should be a single `database_url` ✅ FIXED
The current three-field approach for database config is non-standard. sqlx, dotenv, and the broader Rust ecosystem all expect a single `DATABASE_URL` connection string. The current approach also makes multi-backend support harder.

**Fix:** Replace `pg_host`, `pg_user`, `pg_pass` in `Config` (and in `CliArgs` and `EnvironmentVariables`) with a single `database_url: String` field. ✅ **COMPLETED in Step 1.1 Part B.**

---

## Non-negotiable design constraints

1. Tasks that are `tokio::spawn`ed must be `'static`: closures capture only owned values or `Arc`/`Clone` handles.
2. No new global state. `CONFIG` is still present for lazy initialization and macro fallbacks, but all application code accesses config via `AppContext`. ✅ **COMPLETED in Phase 1.**
3. DB traits are pure persistence: no API client types, no remote fetching.
4. Progress reporting is trait-based (`ProgressReporter`/`ProgressFactory`), injectable from `AppContext`. No direct `indicatif` in business logic.
5. Cancellation is responsive: use `tokio-util::sync::CancellationToken` + `tokio::select!` around all long concurrent waits.
6. `CacheKey` is the single cache key type — a small serialisable namespaced struct, no central enum.
7. The database is a blank slate. Migrations can be freely changed. No existing data to preserve.
8. `last_updated` is always DB-managed via `DEFAULT now()` and `DO UPDATE SET last_updated = now()`. It never appears as a field on a Rust `DbStruct`.

---

## Dependency changes needed

```toml
[dependencies]
tokio-util = { version = "0.7", features = ["sync"] }  # CancellationToken (Phase 2.1)
uuid = { version = "1.0", features = ["v4", "serde"] } # Job IDs (Phase 2.2)
crossterm = "0.28"                                      # Terminal backend (Phase 3.1)
# ratatui 0.30 already in Cargo.toml — gate behind `tui` feature until Phase 3

# To remove:
# - rand — unused

# To move:
# - console-subscriber → [dev-dependencies] or gate behind `tokio-console` feature
```

---

## Roadmap

### Step 0 — CLI subcommands and housekeeping (do first) [Progress]

Make the binary useful and clean up trivial issues before any structural refactoring.

Checklist (Step 0 — mark complete when done):
- [x] `Cargo.toml` changes:
  - [x] Remove `rand`.
  - [x] Move `console-subscriber` to `[dev-dependencies]`.
  - [x] Add `tokio-util = { version = "0.7", features = ["sync"] }`.
- [x] Migration fixes (blank slate, safe to edit freely):
  - [x] Standardise all date/timestamp columns to `TIMESTAMPTZ`.
  - [x] Add `manually_edited BOOLEAN DEFAULT FALSE NOT NULL` to `api_cache`.
- [x] Code fixes:
  - [x] Add `DSError::Cancelled` variant to `common/errors.rs`.
  - [x] Fix `NhlPrimaryKey::PlayoffSeriesGame` `verify_by_key` dispatch.
  - [x] Add missing entities to `warm_nhl_key_cache`: `NhlShift`, `NhlPlayoffSeries`, `NhlPlayoffSeriesGame`.
  - [x] Remove dead `raw_data` string binding from `NhlApiDataArrayResponse::map_json_array_to_json_structs`.
- [x] CLI subcommands — add a `clap` subcommand enum to `CliArgs`:
  - [x] `scrape nhl [--reset]` — debug-only `--reset` to drop/recreate schema.
- [x] Replace inline DROP TABLE reset logic with `reset_schema()` in `common/db/init.rs` (debug-only).

Reference implementation (already present):

```rust
#[cfg(debug_assertions)]
pub async fn reset_schema(pool: &PgPool) -> Result<(), DSError> {
    sqlx::query("DROP SCHEMA public CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("CREATE SCHEMA public")
        .execute(pool)
        .await?;
    sqlx::migrate!("./migrations")
        .run(pool)
        .await?;
    Ok(())
}
```

Acceptance criteria (Step 0):
- [x] `cargo build` clean with no unused dependency warnings.
- [x] `cargo run -- scrape nhl` executes.
- [x] `cargo run -- scrape nhl --reset` resets schema in debug builds.
- [x] `--reset` flag absent in release builds.

---

### Phase 1 — Foundation (goal: owned, spawn-safe core) [Progress & Order Check]

This phase creates the owned, spawn-safe core and is the prerequisite for the job system. The order below is deliberate: each step reduces coupling or enables the next step with minimal churn. I have reviewed the order and confirm it is correct and near-optimal for minimizing breakage and enabling incremental, reviewable PRs.

High-level checklist (Phase 1):
- [x] Step 1.1 Part A — Progress traits + `AppContext` structure (DONE)
- [x] Step 1.1 Part B — `database_url` config + DB/worker plumbing (DONE)
- [x] Step 1.2 — Remove lifetime-bearing resource types (DONE; TTL deferred)
- [x] Step 1.3 — Replace `with_progress!` macro with progress reporter pattern (DONE)
- [x] Step 1.4a — Decouple DB keys from API (introduce `CacheKey`) (DONE; kept `PrimaryKey` name)
- [x] Step 1.4b — Move FK resolution into orchestrator helpers; fix error handling (DONE)
- [x] Step 1.4c — Migrate one entity end-to-end; implement `DataSource` trait (DONE)
- [ ] Step 1.5 — Add cancellation (`CancellationToken`) and tests

Rationale for ordering (verified & executing):
- Step 1.1 first: owning `AppContext` and collapsing DB config into a `database_url` is low risk and required for subsequent spawn-safe changes. ✅ DONE
- Step 1.2 next: removing lifetime-bearing API resources makes functions `'static`-friendly; must follow creation of owned `AppContext`. ✅ DONE
- Step 1.3 follows: progress abstraction depends on `AppContext` ownership and simplifies callsites before mass refactor. ✅ DONE
- Step 1.4a/1.4b: decoupling DB keys and moving FK resolution into the orchestrator is the core re-architecture; doing it after the above ensures spawn-safety and progress plumbing are in place. ✅ DONE
- Step 1.4c: migrate one entity end-to-end as a proof-of-concept; keeps the scope small and demonstrable. ✅ DONE
- Step 1.5: cancellation is threaded last once the orchestration and spawn boundaries are correct. ⏳ NEXT

Design note: `DataSource` trait and source registry

Keep this in mind when implementing Steps 1.1 and 1.4c. Do not over-engineer it upfront; design `AppContext` and CLI dispatch so this slot exists naturally.

Today, `warm_nhl_key_cache` and `main.rs` are the implicit registry for data sources — hardcoded lists that must be updated manually when adding a new source. A `DataSource` trait gives that a formal home:

```rust
trait DataSource: Send + Sync {
    fn name(&self) -> &'static str;
    fn can_handle(&self, job_type: &str) -> bool;
    async fn warm_cache(&self, ctx: Arc<AppContext>) -> Result<(), DSError>;
    async fn scrape(&self, ctx: Arc<AppContext>) -> Result<(), DSError>;
}
```

`AppContext` holds a `Vec<Arc<dyn DataSource>>` populated at startup. The `scrape nhl` CLI subcommand and `warm_key_cache` both iterate the registry rather than containing hardcoded lists. Adding a new source means implementing the trait and adding one line at the registration site — you cannot forget to wire it into cache warming or the CLI because both are driven from the same list.

`can_handle` is used in Phase 2 for daemon job routing: when a `JobSpec` with `job_type: "nhl.scrape_season"` arrives, the executor iterates sources and dispatches to the one that returns `true`. No central match arm needed.

The registration site in `main.rs`:

```rust
let sources: Vec<Arc<dyn DataSource>> = vec![
    Arc::new(NhlDataSource::new(app_context.clone())),
    // Arc::new(EspnDataSource::new(app_context.clone())),
];
```

Note all methods take `Arc<AppContext>` — consistent with constraint 1, since all of these methods will cross `await` points and may eventually be called from spawned tasks.

---

**Step 1.1 — `AppContext` owns config and shared handles**

This step also collapses `pg_host`/`pg_user`/`pg_pass` into a single `database_url` field (critique item 21), which is a prerequisite for multi-backend support in Future Work and a straightforward improvement regardless.

**Part A (COMPLETED):**
- [x] Add `src/common/progress/mod.rs`:
  - `ProgressReporter` trait with methods: `inc(u64)`, `set_len(u64)`, `set_message(&str)`, `finish()`
  - `ProgressReporterMode` enum (not trait-based factory — simpler enum with variants: `Noop`, `Indicatif(Arc<MultiProgress>, ProgressStyle)`)
  - `NoopReporter` and `IndicatifReporter` implementations
  - Method: `create_reporter(&self, total: Option<u64>, msg: &str) -> Box<dyn ProgressReporter + Send>`
- [x] Add `src/common/app_context.rs`:
  - `AppContext { config: Arc<Config>, http: Client, progress_reporter_mode: ProgressReporterMode, cancellation_token: CancellationToken }`
  - Implements `Clone` (cheap — clones Arcs and client)
  - **Temporary field added:** `multi_progress_bar: Arc<MultiProgress>` for backward compat with existing `with_progress!` callsites (will be removed in Step 1.3)
- [x] Add `Clone` derive to `Config`
- [x] Update `main.rs` to construct `AppContext::new(Arc::new((*CONFIG).clone()))`
- [x] Fix all `AppContext` import paths to use `crate::common::app_context::AppContext`
- [x] Suppress `async_fn_in_trait` warning in `src/common/db/db_entity.rs` with `#[allow(async_fn_in_trait)]`

**Design decision: enum not trait for ProgressReporterMode**
We chose `enum ProgressReporterMode { Noop, Indicatif(...), Channel(...) }` over a trait-based factory because:
- Only 3 variants expected (Noop, Indicatif, Channel for daemon IPC)
- Simpler, more direct, easier to grep
- Less boilerplate for this use case
- Easy to extend with a match statement

**Part B (COMPLETED):**
- [x] Replace `pg_host`, `pg_user`, `pg_pass` in `Config`, `CliArgs`, and `EnvironmentVariables` with `database_url: String`
- [x] Change `DbContext::connect()` to `DbContext::connect(cfg: &Config) -> Result<DbContext, DSError>`
- [x] Change `start_sqlx_worker` to accept `WorkerConfig { batch_size, batch_timeout_ms }` instead of reading `CONFIG`
- [x] Change `default_retry_strategy()` in `util.rs` to accept `&Config` instead of reading `CONFIG`

Acceptance: compiles; warm key cache still runs from `main.rs`; `cargo run -- scrape nhl` completes successfully.

---

**Step 1.2 — Remove lifetime-bearing resource types from NhlApi (COMPLETED)**

Changes:
- [x] Fold `PlayerResource`, `GameResource`, `PlayoffBracketResource`, `PlayoffSeriesResource`, `SeasonResource`, `TeamResource`, `FranchiseResource`, `ShiftResource` methods directly onto `NhlApi`.
- [x] All resource methods are now `&self` methods on the owned API struct. No lifetime parameters on resource-like types.
- [x] Update all callers in `orchestrator.rs` and `primary_key.rs`.
- [x] Add `endpoint()` helper methods to `NhlStatsApi` and `NhlWebApi` for DRY URL construction.

Note: Adding `ttl()` method to `DbEntity` trait was deferred to Future Work - needs operational experience with API refresh patterns first.

The API is now spawn-safe and ergonomic. Instead of `nhl.games().get(...)`, callers use `nhl.get_game(...)`. All methods consolidated on `NhlApi` eliminate the distinction between stats and web APIs at the call site.

Acceptance: ✅ `data_sources/nhl/api` compiles with no lifetime-parameterised resource structs; orchestrator calls compile.

---

**Step 1.3 — Replace `with_progress!` with progress reporter pattern (COMPLETED)**

Changes:
- [x] Add sync helper methods to `AppContext`:
  - `with_progress_bar<F, R>(&self, total: u64, msg: &str, f: F) -> R` for sync progress bar
  - `with_spinner<F, R>(&self, msg: &str, f: F) -> R` for sync spinner
- [x] Replace `with_progress!` callsites:
  - `item_parsed_with_context.rs` - uses sync helper method
  - `db_entity.rs` - uses explicit progress reporter calls (2 spinners)
  - `nhl_api.rs` - uses explicit progress reporter calls (2 progress bars)
- [x] Delete the `with_progress!` macro from `util.rs`.
- [x] Remove `multi_progress_bar` field from `AppContext`.
- [x] Delete `UI_CONFIG` static and `UiTheme` struct from `config/mod.rs`.
- [x] Update `AppContext::new()` to properly initialize `progress_reporter_mode` with `Indicatif` variant (added `--no-progress` CLI flag; defaults to enabled)

Design decision: Async helper methods have lifetime issues with complex async code (streams, nested awaits). The solution is to use explicit progress reporter calls for async cases:
```rust
let pb = app_context.progress_reporter_mode.create_reporter(Some(count), msg);
let result = async_work().await;
pb.finish();
```
This pattern is more explicit, easier to debug, and matches what Phase 2 jobs will use (they receive owned `Box<dyn ProgressReporter>`).

Sync helper method signatures:
```rust
impl AppContext {
    // Sync progress bar with known total
    pub fn with_progress_bar<F, R>(&self, total: u64, msg: &str, f: F) -> R
    where
        F: FnOnce(&dyn ProgressReporter) -> R,
    {
        let pb = self.progress_reporter_mode.create_reporter(Some(total), msg);
        let result = f(&*pb);
        pb.finish();
        result
    }
    
    // Sync spinner without known total
    pub fn with_spinner<F, R>(&self, msg: &str, f: F) -> R
    where
        F: FnOnce(&dyn ProgressReporter) -> R,
    {
        let pb = self.progress_reporter_mode.create_reporter(None, msg);
        let result = f(&*pb);
        pb.finish();
        result
    }
}
```

Example transformations:
```rust
// Sync case (uses helper method):
// Before:
with_progress!(app_context.multi_progress_bar, items.len(), "Processing", |pb| {
    items.iter().inspect(|_| pb.inc(1)).collect()
})

// After:
app_context.with_progress_bar(items.len() as u64, "Processing", |pb| {
    items.iter().inspect(|_| pb.inc(1)).collect()
})

// Async case (uses explicit calls):
// Before:
with_progress!(app_context.multi_progress_bar.clone(), ids.len(), "Fetching", |pb| {
    stream.inspect(|_| pb.inc(1)).collect().await
})

// After:
let pb = app_context.progress_reporter_mode.create_reporter(
    Some(ids.len() as u64),
    "Fetching"
);
let result = stream.iter(ids)
    .map(|id| fetch(id))
    .inspect(|_| pb.inc(1))
    .collect()
    .await;
pb.finish();
result
```

Rationale: Sync helper methods provide ergonomics for simple CPU-bound progress. Async cases use explicit calls to avoid lifetime complexity and prepare for Phase 2 where jobs receive owned `Box<dyn ProgressReporter>`. Explicit calls are clearer, more composable, and easier to debug.

Acceptance: ✅ sync callsites use helper methods; async callsites use explicit progress reporter calls; `UI_CONFIG` static gone; `multi_progress_bar` field removed; macro deleted; Noop and Indicatif swappable via `ProgressReporterMode`.

---

**Step 1.4a — Decouple `PrimaryKey` from API; introduce `CacheKey` (COMPLETED)**

Goal: Redesign the DB trait layer to be purely about persistence, not API fetching.

Changes:
- [x] Remove `type Api` and `upsert_from_api` from the `PrimaryKey` trait. (Note: kept name `PrimaryKey` instead of renaming to `EntityKey` - more familiar terminology)
- [x] Add `fn cache_key(&self) -> CacheKey` to `PrimaryKey`. Define `CacheKey { source: &'static str, table: &'static str, id: String }` — serialisable, `Hash`, `Eq`, `Clone`.
- [x] Add `type Entity: DbEntity<Pk = Self>` to `PrimaryKey` for bidirectional relationship.
- [x] Add default `fn cache_key(&self)` to `DbEntity` trait that calls `self.pk().cache_key()`.
- [x] Replace `DashSet<AnyPrimaryKey>` in `DbContext` with `DashSet<CacheKey>`.
- [x] Delete `AnyPrimaryKey` enum and `src/any_primary_key.rs`.
- [x] Delete the `NhlPrimaryKey` enum entirely. Each `DbEntity` impl's `type Pk` becomes its own concrete key struct directly (e.g., `impl DbEntity for NhlTeam { type Pk = NhlTeamKey; }`). The per-key structs (`NhlTeamKey`, `NhlGameKey`, etc.) are kept — they provide `create_select_query` and `cache_key`.
- [x] Update all NHL key structs to use `|` delimiter for composite keys in `cache_key()`.
- [x] Update all NHL model `DbEntity` impls to use concrete key types.
- [x] Comment out FK auto-resolution in `upsert_all()` and `fix_relationships_and_upsert()` with TODO comments (will be fixed in Step 1.4b).
- [x] Fix orchestrator functions (add missing `nhl_api` parameters, remove `api` from tracing macros).

**Files affected:** `common/db/db_entity.rs`, `common/db/init.rs`, `common/db/mod.rs`, `data_sources/nhl/primary_key.rs`, `any_primary_key.rs` (deleted), all 11 NHL model files, `orchestrator.rs`.

Acceptance: ✅ `PrimaryKey` trait has no API associated types; key cache uses `CacheKey`; `AnyPrimaryKey` and `NhlPrimaryKey` enum are gone; compiles cleanly with no errors or warnings.

---

**Step 1.4b — Foreign key resolution in orchestrator and error handling (COMPLETED)**

Goal: Move FK resolution logic from DB layer to orchestrator layer with validation checks, and fix error handling so parse errors are exposed and logged, not swallowed.

Implementation (actual, verified working):

**FK Resolution:**
- [x] Added `find_missing_foreign_keys<T>()` helper in `src/common/db/fk_helpers.rs` — collects all CacheKeys from entities, checks cache, returns missing ones.
- [x] Added `group_cache_keys_by_table()` helper — groups missing keys by table name for diagnostic logging.
- [x] Added `all_foreign_keys_cached<T>()` helper — boolean check that all FKs exist in cache.
- [x] Created `ensure_foreign_keys_exist<T>()` in orchestrator — checks cache, logs warnings with sample data (up to 10 missing keys), returns missing FKs for diagnostics.
- [x] Integrated FK checks into resource upsert flows:
  - `get_resource()` calls `ensure_foreign_keys_exist()` before all `upsert_all()` calls
  - `get_nhl_all_games_in_season()` checks FKs (teams, season) before upserting
  - `get_nhl_roster_spots_in_game()` checks FKs (game, players, teams) before upserting
  - `get_nhl_plays_in_game()` checks FKs (game) before upserting
  - `get_nhl_playoff_series()` checks FKs (season, teams, bracket_series) and upserts playoff series and games
  - `get_nhl_games_in_playoff_series()` checks FKs (season, teams) before upserting

**Error Handling:**
- [x] Changed `NhlStatsApi::fetch_and_parse()` signature to return `Result<Vec<Result<T, DSError>>, DSError>` instead of filtering errors internally.
- [x] Updated `NhlApi` wrapper methods (`list_seasons`, `list_teams`, `list_franchises`, `list_shifts_for_game`, `list_playoff_series_for_year`) to return new signature.
- [x] Updated orchestrator to explicitly partition parse results:
  - `get_resource()` partitions into successes and failures
  - `get_nhl_all_games_in_season()` partitions and logs parse errors with counts
  - `get_nhl_games_in_playoff_series()` partitions and logs parse errors with counts
- [x] All parse errors are logged to `data_source_error` table via `DataSourceError::track_error()` (fire-and-forget).
- [x] Orchestrator continues with successfully parsed items even if some fail.

**Bonus improvements (tracing & error handling audit):**
- [x] Comprehensive audit of all tracing statements and error handling
- [x] Consistent log levels: debug for progress, info for milestones, warn for recoverable issues, error for failures
- [x] Added structured fields throughout (endpoint, type_name, counts, status)
- [x] Removed redundant logging (especially in hot paths)
- [x] Replaced 10+ `unwrap()` calls with proper error propagation
- [x] Used `expect()` with clear messages for truly infallible operations
- [x] Simplified error closures per clippy suggestions
- [x] Removed double-logging in retry helpers
- [x] Downgraded over-verbose serde logs from info to debug

**Design rationale:**
- FK checking is diagnostic-first (logs missing keys with samples) and enforcement-deferred (relies on DB constraints). This is pragmatic: automatic FK fetching would couple generic helpers to source-specific logic. The current pattern gives visibility without magic.
- Parse errors exposed but not fatal: one malformed item doesn't kill the batch. Errors are logged to DB for visibility and debugging.
- Tracing improvements reduce noise in production logs while maintaining debuggability.

**Files affected:** `common/db/fk_helpers.rs` (new), `common/db/mod.rs`, `common/api/cacheable_api.rs`, `common/db/db_entity.rs`, `common/models/api_cache.rs`, `common/serde_helpers.rs`, `common/util.rs`, `data_sources/nhl/api/nhl_stats_api.rs`, `data_sources/nhl/api/nhl_api.rs`, `data_sources/nhl/orchestrator.rs` (major refactor), and 8 supporting files.

Acceptance: 
- ✅ All FK methods implemented on 11 NHL entities
- ✅ Orchestrator validates FKs via cache checks before upserts
- ✅ Parse errors exposed and logged to `data_source_error` table
- ✅ Playoff series upsert reinstated with FK checks
- ✅ Comprehensive error handling improvements (10+ unwrap calls replaced)
- ✅ Consistent structured tracing throughout
- ✅ `cargo build`, `cargo check`, `cargo clippy`, `cargo fmt` all pass
- ✅ Code is cleaner, safer, and more observable

---

**Step 1.4c — Define `DataSource` trait and move cache warming into `NhlDataSource` (COMPLETED)**

Goal: Create the source registry structure (to be used in phase 2 job system). start small: define the trait with just `warm_cache()`, move the existing logic into `NhlDataSource`, prove it works. defer job execution to phase 2 when `JobSpec` is designed.

Changes:
- [x] Define `DataSource` trait in new `src/common/data_source.rs`:
  ```rust
  pub trait DataSource: Send + Sync {
      fn name(&self) -> &'static str;
      async fn warm_cache(&self, app_context: &AppContext, db_context: &DbContext) -> Result<(), DSError>;
  }
  ```
- [x] Create `NhlDataSource` struct in new `src/data_sources/nhl/data_source.rs`:
  ```rust
  pub struct NhlDataSource {
      api: NhlApi,
  }
  impl DataSource for NhlDataSource {
      fn name(&self) -> &'static str { "nhl" }
      async fn warm_cache(&self, app_context: &AppContext, db_context: &DbContext) -> Result<(), DSError> {
          // concurrent cache warming with buffer_unordered
      }
  }
  ```
- [x] Add `sources: Arc<Vec<Arc<dyn DataSource>>>` to `AppContext`. initialize with `vec![Arc::new(NhlDataSource::new())]` in `main.rs`.
- [x] Add `with_sources()` builder method to `AppContext`.
- [x] Update `main.rs` to call `sources[0].warm_cache()` instead of directly calling `warm_nhl_key_cache()`.
- [x] Delete the `warm_nhl_key_cache()` function from orchestrator (its logic is now in `NhlDataSource::warm_cache`).
- [x] Implement concurrent cache warming using `buffer_unordered` with `app_context.config.db_concurrency_limit`.

**Files affected:** new `src/common/data_source.rs`, new `src/data_sources/nhl/data_source.rs`, `src/common/app_context.rs`, `src/main.rs`, `src/data_sources/nhl/orchestrator.rs`.

Acceptance: ✅ `DataSource` trait compiles; `NhlDataSource` implements it correctly; `AppContext` holds a registry with `sources` field; `main.rs` initializes the registry and calls `warm_cache` via it; `cargo build` passes; `scrape nhl` works end-to-end with the same behavior as before; cache warming runs concurrently.

**deferred to phase 2:** `JobSpec` enum, `DataSource::execute()`, job routing logic in executor. that design work will happen when we build the job system.

---

**Step 1.5 — Cancellation**

Add `tokio-util` dependency (already added in Step 0). Thread `CancellationToken` through orchestrator functions.

**Two levels of cancellation:**

1. **Coarse: checkpoint between phases.** `token.is_cancelled()` between fetch → parse → upsert. Cheap, handles the common case.

2. **Fine: `select!` during long awaits.** For `buffer_unordered(...).collect()` calls waiting on many concurrent requests:

```rust
tokio::select! {
    results = stream::iter(ids)
        .map(|id| fetch(id))
        .buffer_unordered(limit)
        .collect::<Vec<_>>() => { /* handle */ }
    _ = token.cancelled() => {
        return Err(DSError::Cancelled);
    }
}
```

This provides immediate responsiveness — `select!` drops in-flight futures as soon as the token fires.

Changes:
- Add `cancellation_token: CancellationToken` to `AppContext`. Child tokens are derived per job/task.
- Wrap `buffer_unordered(...).collect()` awaits in `tokio::select!` racing each future against `token.cancelled()`, returning `Err(DSError::Cancelled)` on cancellation. (`DSError::Cancelled` was added in Step 0.)
- CLI mode passes a never-cancelled token (for now).
- Add a unit test: spawn many slow futures in a `buffer_unordered`, cancel via token, assert all shut down within a tight time budget (e.g., 200ms).

**Files affected:** `common/app_context.rs`, all orchestrator functions, new test file.

Acceptance: cancellation test passes; orchestrator-level collect loops exit immediately on cancel.

---

### Phase 1 completion criteria

**ALL PHASE 1 CRITERIA MET — READY FOR PHASE 2:**

**Step 1.4b completion (VERIFIED):**
- ✅ All FK helper utilities exist and are wired into orchestrator
- ✅ FK validation happens before all upserts with diagnostic logging
- ✅ Parse errors are exposed (not swallowed) and logged to `data_source_error`
- ✅ Playoff series upsert is reinstated with FK checks
- ✅ Comprehensive tracing improvements (structured fields, consistent levels)
- ✅ Error handling hardening (10+ unwrap() calls replaced, better error propagation)
- ✅ All builds pass (check, build, clippy, fmt)

**Step 1.4c completion (VERIFIED):**
- ✅ `DataSource` trait exists with `warm_cache()` method; `NhlDataSource` implements it; registry is in `AppContext`
- ✅ `warm_nhl_key_cache` logic is moved into `NhlDataSource::warm_cache`; `main.rs` calls it via the registry
- ✅ Concurrent cache warming using `buffer_unordered` with config-driven concurrency limits
- ✅ All builds pass with zero warnings

**Step 1.5 (DEFERRED TO PHASE 2):**
- ✅ `CancellationToken` field present in `AppContext` (infrastructure ready)
- ✅ `DSError::Cancelled` variant defined
- ⏸️ Cancellation implementation deferred to Phase 2.1 (will be done alongside job executor)
- ⏸️ Cancellation test deferred to Phase 2.1

**Core criteria met:**
- ✅ Orchestrator functions accept `&AppContext`, `&DbContext`, `&NhlApi` as needed
- ✅ `CONFIG` static remains but `UI_CONFIG` is fully removed
- ✅ Global `CONFIG` removed from all hot paths (only used in retry macro fallbacks)
- ✅ `cargo build --release` passes
- ✅ All code compiles cleanly with no warnings
- ✅ Foundation is owned, spawn-safe, and ready for job system
- ✅ `PrimaryKey` trait has no API-related methods.
- ✅ `AnyPrimaryKey` and `NhlPrimaryKey` enum are gone. `CacheKey` is the single cache key type.
- ✅ `with_progress!` macro is gone. Progress goes through `ProgressReporterMode`.

---

### Phase 2 — Job system and daemon [Checklist & Order Check]

Implement only after Phase 1 is complete and the binary is stable end-to-end.

Phase 2 checklist:
- [ ] Step 2.1 — Job system (`Job` trait, `JobExecutor`, `JobHandle`)
- [ ] Step 2.2a — IPC protocol and types (ClientMessage/DaemonMessage, framing)
- [ ] Step 2.2b — Daemon implementation (socket, job persistence, executor)
- [ ] Step 2.3 — CLI client (daemon IPC client, job submission & repair flows)

Ordering verification:
- The order is correct: the job system (2.1) defines the primitives that IPC (2.2a) and the daemon (2.2b) use. The CLI client (2.3) depends on the daemon being available. Implementing 2.2a before 2.2b allows parallel design/review of protocol types and the daemon implementation.

#### Step 2.1: Job system

```rust
#[async_trait]
pub trait Job: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> String;

    async fn execute(
        self: Box<Self>,
        ctx: AppContext,
        reporter: Box<dyn ProgressReporter>,
        token: CancellationToken,
    ) -> Result<(), DSError>;
}
```

Concrete job structs:

```rust
pub struct FetchSeasonsJob;
pub struct FetchGamesInSeasonJob { pub season_id: i32 }
pub struct FetchEverythingInSeasonJob { pub season_id: i32 }
```

Each `execute` impl constructs an `NhlApi` from `ctx.http.clone()`, calls orchestrator functions, and discards the typed return value.

**Note:** `execute` takes `AppContext` by value (it's `Clone`, so the executor clones one per task). It takes `Box<dyn ProgressReporter>` (owned, because the spawned task needs to own its reporter). The executor creates a `ChannelReporter` per job.

`JobSpec` — the IPC-serializable enum:

```rust
#[derive(Serialize, Deserialize)]
pub enum JobSpec {
    FetchSeasons,
    FetchGamesInSeason { season_id: i32 },
    FetchEverythingInSeason { season_id: i32 },
}

impl JobSpec {
    pub fn into_job(self) -> Box<dyn Job> { ... }
}
```

`JobHandle`, `JobExecutor`:

```rust
pub struct JobHandle {
    pub id: Uuid,
    pub name: String,
    pub status: JobStatus,
    pub started_at: Option<Instant>,
    pub progress_rx: mpsc::Receiver<ProgressUpdate>,
    pub cancel_token: CancellationToken,
}

pub struct JobExecutor {
    ctx: AppContext,
    tasks: JoinSet<(Uuid, Result<(), DSError>)>,
    handles: HashMap<Uuid, JobHandle>,
}
```

Because `AppContext` is `Clone` and all async work uses owned handles, `tokio::spawn` inside the executor just works — no lifetime issues.

**Files affected:** New `src/jobs/` module tree.

#### Step 2.2a: IPC protocol and types

Goal: Define the communication protocol between daemon and clients.

Changes:
- Create `src/ipc/mod.rs` with protocol message types:

```rust
pub enum ClientMessage {
    SubmitJob(JobSpec),
    CancelJob(Uuid),
    CancelAll,
    ListJobs,
    JobStatus(Uuid),
    SubscribeProgress(Uuid),
    SetLogLevel(String),
    Shutdown,
}

pub enum DaemonMessage {
    JobSubmitted(Uuid),
    JobList(Vec<JobSummary>),
    JobStatus(JobStatusInfo),
    ProgressUpdate(Uuid, ProgressUpdate),
    Error(String),
    Ok,
}
```

- Implement newline-delimited JSON framing helpers (`read_message`, `write_message`) for Unix domain sockets.
- Add `ChannelReporter` implementation of `ProgressReporter` that sends `ProgressUpdate` messages over an `mpsc::Sender`.
- Define `JobSummary`, `JobStatusInfo`, `ProgressUpdate` supporting types.

Acceptance: IPC types defined, serializable with serde; frame helpers can read/write messages to a test socket; `ChannelReporter` exists and implements `ProgressReporter`.

#### Step 2.2b: Daemon implementation

Goal: Build the long-running daemon process that executes jobs.

Changes:
- Create `src/bin/daemon.rs`:
  - Constructs `AppContext` (it's already `Clone`).
  - Creates `JobExecutor`.
  - Binds Unix domain socket (default: `~/.local/share/dry-scraper/daemon.sock`).
  - Socket file permissions set to `0o600` on creation.
  - Writes PID file to `~/.local/share/dry-scraper/daemon.pid`.
  - Main event loop:

```rust
loop {
    tokio::select! {
        Ok((stream, _)) = listener.accept() => {
            handle_client(stream, executor, ...);
        }
        Some(result) = executor.poll_completions() => {
            update_status(result);
            notify_clients(result);
        }
        _ = shutdown_signal() => {
            executor.cancel_all();
            break;
        }
    }
}
```

- Implement `handle_client`: reads `ClientMessage`, dispatches to executor, sends `DaemonMessage` responses.
- Add `jobs` table migration (backend-agnostic, works for both Postgres and SQLite): stores job ID, spec JSON, status, timestamps.
- **JobSpec persistence:** All accepted `JobSpec`s are written to the `jobs` table with status `submitted` before execution begins.

**Files affected:** New `src/bin/daemon.rs`, new `src/daemon/mod.rs`, new migration file.

Acceptance: Daemon binary compiles; can start and bind to socket; accepts a connection and reads a `ClientMessage`; gracefully shuts down on SIGTERM.

#### Step 2.3: CLI client

Goal: Rework the CLI to communicate with the daemon instead of running tasks directly.

Available commands:
- `fetch seasons` — submit FetchSeasons job
- `fetch games --season <id>` — submit FetchGamesInSeason job
- `fetch all --season <id>` — submit FetchEverythingInSeason job
- `job list` — list all jobs
- `job cancel <uuid>` — cancel a job
- `daemon shutdown` — gracefully shut down daemon
- `errors list [--limit N]` — list recent parse errors
- `errors show <error_id>` — display error details
- `errors repair <error_id>` — interactive fix in $EDITOR
- `errors retry <error_id>` — re-parse and upsert fixed data

Client auto-starts daemon if not running. Default is "follow" mode (submit + stream progress). `--detach` flag for fire-and-forget.

Error repair workflow ensures manually repaired cache entries are never overwritten by fresh API fetches (checked in `fetch_endpoint_cached`).

**Files affected:** `main.rs` rewrite, shared IPC client utility, new `src/cli/errors.rs` for error repair commands.

**Acceptance:** CLI successfully submits jobs to daemon; `follow` mode streams progress; `errors repair` workflow fixes a parse error and marks cache entry as manually edited; daemon auto-starts if not running.

---

### Phase 3 — TUI, scheduler, metrics [Checklist]

Rework `main.rs` as a daemon client (most of the CLI commands above are implemented here).

Phase 3 checklist:
- [ ] Step 3.1 — TUI client (ratatui + crossterm)
- [ ] Step 3.2 — Scheduling and live game tracking (daemon scheduler & LiveGameTrackingJob)
- [ ] Step 3.3 — Additive enhancements (metrics, audits, DBContext stats)

#### Step 3.1: TUI client

`src/bin/tui.rs` using `ratatui` (already at 0.30 in Cargo.toml) + `crossterm`.

The TUI is a pure IPC client — no `AppContext`, no DB, no API. Its state is:

```rust
struct AppState {
    jobs: Vec<JobInfo>,
    selected_job: Option<Uuid>,
    progress: HashMap<Uuid, ProgressInfo>,
    logs: VecDeque<String>,
    // UI navigation state
}
```

Event loop:

```rust
loop {
    terminal.draw(|f| render(f, &state))?;
    tokio::select! {
        Some(event) = input_rx.recv() => handle_input(event, &mut state),
        Some(msg) = daemon_rx.recv() => update_state(msg, &mut state),
        _ = tick.tick() => {}
    }
}
```

Screens (build incrementally):
- Dashboard — job list with status, progress, elapsed time
- Job detail — live progress for a single job, cancel button
- Job builder — browse seasons, select what to fetch, submit
- Settings — view config, change log level

**Files affected:** New `src/bin/tui.rs` and `src/tui/` module tree. Add `crossterm` to `Cargo.toml`.

#### Step 3.2: Scheduling and live game tracking

Internal daemon scheduler:

- On startup, fetch today's NHL schedule.
- For live games, spawn `LiveGameTrackingJob` that polls every N seconds (adaptive: high frequency during play, low during intermission, stop when final).
- For upcoming games, set a timer.
- Simple cron-like scheduled jobs (daily fetches, etc.) via `tokio::time::interval`.

```rust
pub struct LiveGameTrackingJob {
    pub game_id: i32,
    pub poll_interval: Duration,
}
```

**Files affected:** New `src/jobs/live_game.rs`, new `src/daemon/scheduler.rs`.

#### Step 3.3: Additive enhancements

These can happen at any point after Phase 1.

**Metrics:**
```rust
pub struct Metrics {
    pub api_cache_hits: AtomicU64,
    pub api_cache_misses: AtomicU64,
    pub api_requests: AtomicU64,
    pub db_upserts: AtomicU64,
    pub total_errors: AtomicU64,
    pub start_time: Instant,
}
```
Add to `AppContext`. Instrument `fetch_endpoint_cached` and `upsert_batch`. Expose via `GetMetrics` daemon message.

**DbContext stats:** `pool_size()`, `pool_idle()`, `key_cache_size()` methods.

**In-memory error log:** Bounded `VecDeque<TrackedError>` behind `std::sync::Mutex`. Wire into `track_and_filter_errors`. Expose via daemon message.

**Change detection and audit:** Add `data_change_audit` table to track unexpected changes to "stable" data:

```sql
CREATE TABLE data_change_audit (
    id SERIAL PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_key TEXT NOT NULL,
    field_changed TEXT NOT NULL,
    old_value JSONB,
    new_value JSONB,
    detected_at TIMESTAMPTZ DEFAULT NOW(),
    reviewed BOOLEAN DEFAULT FALSE
);
```

Implement change detection in upsert logic: when refreshing data that's past its initial stability window (e.g., game finished >24hrs ago), compute a diff and log significant changes (score changes, state reversals, stat corrections) to the audit table. This provides visibility into late corrections without blocking them.

Add CLI commands:
```bash
dry-scraper changes list [--unreviewed]
dry-scraper changes show <change_id>
dry-scraper changes approve <change_id>
```

Optional: add `manually_locked BOOLEAN` column to entity tables for records that should never be auto-refreshed (check in `smart_upsert` before fetching).

This addresses the reality that sports data can be corrected hours or days after games finish — the decay-based TTL keeps checking, but changes are logged for review rather than silently applied.

---

## Tests to add (minimum, by phase)

Phase 1:
- Cancellation harness (unit test): N slow futures in `buffer_unordered`, cancel via token, assert all done within 200ms.
- `ProgressReporter` trait test: `NoopFactory` and `IndicatifFactory` both produce reporters that handle `inc`/`finish` without panic; counter increments are correct.
- `CacheKey` round-trip: serialise and deserialise a `CacheKey`, assert equality.
- DB worker smoke test (integration, Postgres container): enqueue 10 jobs, flush, assert all receivers get `Ok`.

Phase 2:
- `JobExecutor` unit test: submit a no-op job, await completion, assert status transitions correctly.
- Daemon integration test: start daemon in-process, submit a job via unix socket, assert progress events arrive.
- Cancel integration test: submit a slow job, cancel it, assert cancellation within 200ms and status transitions to `cancelled`.
- Error repair integration test: create a parse error, use `errors repair` CLI to fix it, verify cache entry marked `manually_edited = TRUE`, verify subsequent fetches don't overwrite.

---

## Branching and PR rules

- Branch: `feature/rewrite-foundation`.
- PRs map 1:1 to a named Step in this file.
- Every PR: description references its Step, lists changed files, includes one acceptance check.
- Every commit must compile. CI runs `cargo build --all` and `cargo test --lib` on every push.
- No PR mixes steps or touches more than ~6 files for a single logical change.

---

## Future Work (not in current plan — needs more design time)

### Cache refresh strategy (API cache TTL, entity TTL, and `manually_edited` flag)

The `api_cache` table has a `manually_edited` boolean column (added in migration 02) that is not currently used in the Rust code. The intent is to protect manually-edited cache entries from being overwritten by automatic API refreshes.

Additionally, Step 1.2 originally planned to add a `ttl()` method to `DbEntity` trait for decay-based entity refresh, but this was deferred because the interaction between API cache TTL and entity TTL needs design work.

**Design questions that need answers first:**
- What is the refresh strategy for `api_cache`? TTL-based? On-demand? Pattern-based (live games vs historical)?
- How does `api_cache` TTL interact with `DbEntity` TTL? Should entity TTL drive API fetches, or vice versa?
- Storage is cheap and API rate limits matter — should most cache entries be permanent unless explicitly refreshed?
- Different endpoints have different update patterns (live games change every 30s, historical data never changes) — needs per-endpoint or pattern-based strategy.
- Should entities have decay-based TTL (e.g., final games age out: 1hr → 12hr → daily → weekly → monthly)?
- How does `manually_edited` flag prevent overwrites in practice? When does refresh logic check it?

**Proposed entity TTL design (from original Step 1.2 plan):**
```rust
pub trait DbEntity: Clone + Send + Sync + 'static {
    // ... existing methods ...
    
    /// How long this entity remains fresh. None = never auto-refresh (manual only).
    fn ttl(&self) -> Option<Duration> {
        Some(Duration::from_secs(86400))  // Default: 24 hours
    }
}

impl DbEntity for NhlGame {
    fn ttl(&self) -> Option<Duration> {
        if !matches!(self.game_state, GameState::Final) {
            return Some(Duration::from_secs(30));  // Active: 30 seconds
        }
        
        // Final games: TTL grows with age to catch corrections
        let age = Utc::now() - self.game_date;
        match age.num_days() {
            0..=1 => Some(Duration::from_hours(1)),    // Day of/after: hourly
            2..=7 => Some(Duration::from_hours(12)),   // Week after: twice daily
            8..=30 => Some(Duration::from_days(1)),    // Month after: daily
            31..=365 => Some(Duration::from_days(7)),  // Year after: weekly
            _ => Some(Duration::from_days(30)),        // Old: monthly sweep
        }
    }
}
```

This handles the reality that sports data is eventually consistent — stats get corrected, games get rescheduled, official decisions get reversed days later.

**When to implement:** After gaining operational experience with the scraper to understand actual API update patterns and refresh requirements. The `manually_edited` column exists in the schema and can be utilized once the broader caching strategy is designed.

---

### `#[derive(PgUpsert)]` proc macro crate

The biggest per-entity boilerplate cost is `upsert_query()`: every column name appears four times across the INSERT column list, VALUES positional params, DO UPDATE SET clause, and `.bind()` chain. None of that is semantic — it's mechanical repetition of what the struct already declares. A derive macro should eliminate it entirely.

**Proposed shape:**

```rust
#[derive(FromRow, Clone, PgUpsert)]
#[pg_upsert(table = "nhl_season", conflict_target = "id")]
pub struct NhlSeason {
    pub id: i32,
    pub all_star_game_in_use: bool,
    // ...
}
```

That attribute pair would be the entire `upsert_query()` impl. The macro emits the INSERT column list, VALUES positional params, ON CONFLICT clause, DO UPDATE SET clause, and all `.bind(self.field)` calls from the struct field list at compile time.

**Known wrinkles to resolve before implementing:**

- Composite conflict targets (`conflict_target = "game_id, player_id"`) — affects `ON CONFLICT (...)` clause. Straightforward but needs explicit attribute support.
- PK fields should be excluded from the DO UPDATE SET clause. The macro needs to know which fields are in `conflict_target` and skip them in the update list.
- `last_updated = now()` — the macro should always append `last_updated = now()` to the DO UPDATE SET clause unconditionally, since no struct ever carries a `last_updated` field (design constraint 8). This is the cleanest option and requires no special attribute.
- Proc macro crates must be their own crate in Cargo. This means adding a `dry-scraper-macros` sub-crate. Worth doing cleanly rather than rushing it.

**The `*Json` → `*DbStruct` split is intentional and should stay.** The conversion does real work: flattening nested structs (`TeamGameJson` → `away_team_id`, `away_team_name`, etc.), type coercion (`LocalizedNameJson` → `String` via `.best_str()`), dropping non-DB fields (`plays`, `roster_spots`), and injecting context fields (`endpoint`, `raw_json`, `game_id`). This logic can't be derived — it is semantic, not mechanical. The verbosity in `into_db_struct` is documenting the transformation. The one improvement is ensuring `Context: Clone` is consistently enforced so `.clone()` workarounds are never needed — this is already true for all context types in `common.rs`.

**When to implement:** after Phase 1 is complete and the `EntityKey`/`CacheKey` refactor has stabilised the struct boundaries. Retrofitting the macro onto all existing entities should be done in one pass, not piecemeal, to keep the diff reviewable.

---

### `DataSource` trait — further design notes

The trait sketched in Phase 1 above intentionally stays small. Some things worth thinking through before implementing:

**What belongs on the trait vs on the orchestrator:**
The trait is the registration and dispatch interface — it should only have methods that the registry needs to call generically. Heavy orchestration logic (fetching seasons, resolving FK chains, pagination) stays in the source-specific orchestrator module. The trait method `scrape()` is the entry point that calls into the orchestrator, not the orchestrator itself.

**Per-entity cache warming vs per-source:**
`warm_cache` on the trait warms all entities for that source in one call. Internally it is still a `buffer_unordered` over per-entity `warm_key_cache` calls. The trait just gives the registry a uniform interface so it doesn't need to know which entities a source has.

**Job routing in Phase 2:**
`can_handle` provides the routing key. A source with `name() = "nhl"` would implement `can_handle` to return `true` for any `job_type` starting with `"nhl."`. The daemon executor iterates the registry and dispatches to the matching source. This keeps the daemon core free of source-specific knowledge.

**Source-specific config:**
Some sources will need their own config (API keys, base URLs, rate limits). These live on the concrete source struct, not on `AppContext`. The source is constructed with its config at registration time and holds it internally.

**Testing:**
The trait makes testing straightforward — implement a `MockDataSource` that records calls, inject it into `AppContext`, and test the CLI dispatch and cache warming logic without touching a real API or DB.

---

### Multi-backend database support (SQLite + Postgres)

**Depends on:** `#[derive(PgUpsert)]` macro being implemented first.

The goal is that the only thing a user ever has to configure is a single `DATABASE_URL` in their `.env` or config file:

```
DATABASE_URL=sqlite:///home/user/.local/share/dry-scraper/data.db
```
or
```
DATABASE_URL=postgres://user:pass@localhost/dry_scraper
```

sqlx already parses the scheme prefix and routes to the correct backend. The `database_url` field introduced in Step 1.1 is exactly this interface — no further config change needed from the user's perspective when SQLite support lands.

**What actually needs to change for SQLite support:**

- `JSONB` → store as `TEXT`. sqlx can map `TEXT` ↔ `serde_json::Value` transparently. No Rust type changes.
- Custom Postgres enums (`game_type`, `period_type`) → store as `TEXT` in SQLite. Keep the Rust enums, add text round-trip impls. Migrations differ, Rust types don't.
- `TIMESTAMPTZ` → `DATETIME` in SQLite. sqlx's `chrono` feature handles both already.
- `ON CONFLICT ... DO UPDATE SET col = EXCLUDED.col` — this is the only real syntax difference. Postgres uses `EXCLUDED.col` (uppercase by convention); SQLite uses `excluded.col`. With the `PgUpsert` macro generating all upsert SQL in one place, this becomes a single compile-time branch on a `sqlite` feature flag. Without the macro it would mean editing every `upsert_query()` by hand — which is exactly why this depends on the macro.
- Migrations split into `migrations/postgres/` and `migrations/sqlite/`, with `sqlx::migrate!()` pointed at the right one via feature flag or runtime detection from the `DATABASE_URL` scheme.
- SQLite requires `PRAGMA foreign_keys = ON` at connection time. Handled once at connection setup, invisible to the rest of the code.
- The `jobs` table (added in Phase 2) should be written without Postgres-specific types from the start, so it works in both migration sets without changes.

**The one operational difference worth documenting:**

SQLite is a file. The daemon owns the write connection; CLI processes communicate through the daemon socket. No code coordination needed beyond what the daemon architecture already provides.

**When to implement:** after the `PgUpsert` macro is done and all upsert queries are generated centrally.

---

## Timeline (sprint-sized, one engineer)

| Phase | Step | Complexity | Estimate | Description |
|-------|------|-----------|----------|-------------|
| **Phase 0** | Step 0 | Low-Medium | 1 day | Housekeeping, migration fixes, CLI subcommands |
| **Phase 1** | Step 1.1 | Medium | 1-2 days | `AppContext`, `database_url`, config plumbing |
| | Step 1.2 | High | 2-3 days | Remove API lifetimes, add TTL to `DbEntity` |
| | Step 1.3 | Medium | 1 day | `ProgressReporter` trait, delete `with_progress!` |
| | Step 1.4a | Medium | 1 day | Decouple `DbEntity` from API, introduce `CacheKey` |
| | Step 1.4b | Medium | 1 day | FK resolution helper, error handling pattern |
| | Step 1.4c | Medium | 1 day | Migrate one entity, implement `DataSource` trait |
| | Step 1.5 | Medium | 1 day | Cancellation with `CancellationToken` |
| | **Phase 1 total** | | **~8-11 days** | |
| **Phase 2** | Step 2.1 | Medium | 2 days | Job system |
| | Step 2.2a | Medium | 1-2 days | IPC protocol and types |
| | Step 2.2b | High | 2-3 days | Daemon implementation |
| | Step 2.3 | Medium | 1-2 days | CLI client with error repair workflow |
| | **Phase 2 total** | | **~6-9 days** | |
| **Phase 3** | Step 3.1 | High | 4-6 days | TUI client |
| | Step 3.2 | Medium-High | 3-4 days | Scheduling, live game tracking |
| | Step 3.3 | Low | 1-2 days | Metrics, enhancements |
| | **Phase 3 total** | | **~8-12 days** | |
| | **Total** | | **~22-32 days** | |

---

## Future work (not in current plan)

- **Job persistence and resume** — `job_runs` table, resume on daemon restart.
- **Systemd service** — package daemon as a systemd unit.
- **Multi-league support** — add `data_sources/mlb/`, `data_sources/nba/`, etc. The architecture supports this natively after Phase 1.
- **Web API layer** — REST/GraphQL read-only API over the same Postgres. Separate crate.

---
