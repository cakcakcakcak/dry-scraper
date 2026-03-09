# dry-scraper

[![CI](https://github.com/cakcakcakcak/dry-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/cakcakcakcak/dry-scraper/actions)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)

`dry-scraper` is a production-minded Rust data-ingestion tool that fetches NHL data from official APIs and upserts typed records into a Postgres database. It emphasizes reliability, scalable concurrency, observability, and pragmatic data engineering patterns suitable for long-running ingestion jobs.

## Highlights
- DB-backed API cache to avoid redundant network fetches.
- Layered parsing pipeline: raw JSON -> typed JSON structs -> DB-ready domain structs.
- In-memory primary key cache to quickly deduplicate entries before database upserts.
- Batched upserts + worker pattern to reduce contention and increase throughput.
- Configurable retry strategies with jittered exponential backoff for network & DB resilience.
- Observability via `tracing` and user-friendly progress UI (`indicatif`).
- Clear async architecture using Tokio and Reqwest.
- Pragmatic DB integration with SQLx migrations included.
- Thoughtful concurrency controls that separate API and DB limits.
- Extensible design for adding new data sources with minimal coupling.

### Tech stack
- Rust (Tokio runtime)  
- HTTP: `reqwest`  
- DB: `sqlx` (Postgres) + committed `migrations/`  
- Concurrency: `futures`, `tokio`  
- Observability: `tracing`, `tracing-subscriber`, `indicatif`  
- Tooling: `cargo fmt`, `cargo clippy`, `cargo test`  
- CI: GitHub Actions (build, fmt check, clippy, tests)

## Architecture Diagram
```
             +--------------------------------+
             |       External NHL APIs        |
             |                                |
             +---------------+----------------+
                             |
                             v
+----------------------------+----------------------------+
|                      HTTP client layer                  |
|  - retry wrappers, rate-limit aware                     |
|  - location: `src/data_sources/nhl/api/*`               |
+----------------------------+----------------------------+
                             |
             (persist raw JSON responses to `api_cache`)
                             v
                  +-----------------------+       +------------------+
                  |     API cache DB      |<------|   Cache table    |
                  |    (`api_cache`)      |       |   `migrations/`  |
                  +-----------------------+       +------------------+
                             |
                             v
                 +-------------------------+   +-----------------------------+   +------------------+
                 | Orchestrator / Runner   |   |  Parsing & Normalization    |   |  Key warm cache  |
                 | - coordinates fetches   |-->|  - JSON -> typed structs    |-->|  (in-memory)     |
                 | - location:             |   |  - location:                |   |  - reduces DB    |
                 |  `src/data_sources/nhl/`|   |   `src/data_sources/nhl/`   |   |    upserts       |
                 +-------------------------+   +-----------------------------+   +------------------+
                             |
                             v
+------------------------------------------------------------+
|                   DB Upsert Workers                        |
|     - enqueue batched SQLx upserts                         |
|     - worker queue pattern: `src/common/db/worker.rs`      |
|     - retries/backoff on DB transient errors               |
+-----------------------------+------------------------------+
                              |
                              v
                      +---------------+
                      |  Postgres DB  |
                      | (`sqlx` pool) |
                      +---------------+
                              |
                              v
                  +--------------------------+
                  | Observability & Tools    |
                  | - tracing, progress UI   |
                  | - logs & metrics         |
                  | - config: `src/config/*` |
                  +--------------------------+
```

Cross-cutting concerns:
- Concurrency controls: separate API concurrency and DB concurrency (config in `src/config/*`).
- Resilience: retry wrappers with jitter (see `src/common/util.rs`).
- Migrations: committed under `migrations/` and applied with `sqlx::migrate!()`.

## Repository layout
- `src/main.rs` — application entrypoint, tracing, and bootstrap
- `src/config/` — CLI and env var configuration
- `src/common/api/` — Cacheable API abstraction and DB-backed cache
- `src/common/db/` — primary-key cache, worker pattern and upsert helpers
- `src/data_sources/nhl/` — NHL API clients, models, orchestrator
- `migrations/` — SQLx migrations used at runtime

## Quickstart — local development

### Prerequisites
- Rust toolchain (stable), `cargo`
- PostgreSQL database

1) Copy environment template (do NOT commit `.env`) and set required environment variables
```dry-scraper/README.md#L100-108
cp .env.example .env
# Edit `.env` and set DATABASE_URL
```

2) Build & run
```dry-scraper/README.md#L126-132
cargo build --release
cargo run --release
```

### Notes
- On first run the application invokes `sqlx::migrate!()` to apply all table migrations found in `migrations/`.
- The `main` in this repo does not perform live fetches by default. It initializes config, runs migrations, and sets up the ingestion orchestration, but actual network ingestion of NHL APIs is currently disabled. `main` must be modified to utilize the orchestration code and actually perform fetches.



### Configuration
- Primary configuration is assembled from CLI args (via `clap`) and environment variables.
- Minimum required env var:
  - `DATABASE_URL` — Postgres connection string (e.g. `postgres://user:pass@localhost:5432/lp`)

## Future work
- Add unit and integration testing to ensure reliability
- Implement a service mode for scheduled background ingestion
- Implement a TUI for interactive data fetching
- Expand data sources to include other leagues and APIs
- Implement a semaphore-based rate limiter for API and database requests
