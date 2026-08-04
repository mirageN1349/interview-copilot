# History and retention evidence

Automated evidence is recorded by `src-tauri/tests/history_retention.rs` against synthetic SQLite data only.

| Check | Command | Result |
|---|---|---|
| 10,000-row field search, stable cursor, query plan, p95 under 2 s | `cargo test --manifest-path src-tauri/Cargo.toml --test history_retention benchmark_10k_field_searches_and_query_plan -- --nocapture` | Pass: 24 ms p95 on this machine; FTS5 `VIRTUAL TABLE INDEX 0:M7`; primary-key joins for meeting/policy/profile; temporary B-tree for stable final ordering |
| Authorization, detail DTO, deletion, retention, export audit | `cargo test --manifest-path src-tauri/Cargo.toml --test history_retention` | Pass: 5/5 tests |

The benchmark prints the measured p95 and SQLite `EXPLAIN QUERY PLAN` output. Expired content is denied before physical cleanup, while the minimal audit row remains retained for 365 days.
