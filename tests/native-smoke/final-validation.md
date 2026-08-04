# Final signed-application validation

Status: **incomplete — do not mark T070 complete**

Evidence date: 2026-08-04 (Europe/Moscow)

## Completed automated quickstart checks

| Area | Evidence |
| --- | --- |
| Frontend contracts/components | Full Vitest suite passes |
| Type safety and lint | `pnpm typecheck` and `pnpm lint` pass |
| Native storage/security/geometry | Full Rust suite passes |
| Rust static analysis | `cargo clippy --all-targets -- -D warnings` passes |
| Synthetic history benchmark | 10,000 rows, 24 ms p95 on this machine |
| Signed artifacts | Strict verification passes; executable SHA-256 `11cb4ea039c7886467c3290ae36d0e1fdee960ac4158ad2d04ba90728eb5ec98`, DMG SHA-256 `e69f732b49e5ec84d0cbbace561e4afe41597a9d067cb43035565b952641472b` |
| Repeat launch regression | Existing local database opens idempotently; targeted Rust regression and full suites pass, and the debug app remains available on port 1420 |

## Signed validation matrix

| Quickstart section | Status | Evidence document / blocker |
| --- | --- | --- |
| Mock sign-in token reuse | Incomplete | `permissions.md`: packaged diagnostic token retrieval is unavailable. |
| TCC permission restart/revocation | Incomplete | `permissions.md`: controlled signed UI/TCC run not executed. |
| Profile preparation | Automated only | Profile contract/storage/UI tests pass; signed visual path not recorded. |
| Standard meeting | Incomplete | `standard-meeting.md`: native audio, timing, display-loss and retained-media observations missing. |
| Screenshot/data boundary | Automated only | Geometry/redaction tests pass; signed multi-monitor/TCC path not recorded. |
| Keyboard/accessibility | Incomplete | `live-coding-system-design.md`: signed VoiceOver, 200% scale and keyboard path missing. |
| History/retention/audit | Automated | `history-retention.md`: synthetic search/retention/audit evidence passes. |
| Capture feasibility matrix | Blocked | `capture-matrix.md`: no exact signed reference-stream evidence; every row remains disabled. |

## Promotion blockers

- Execute the signed magic-link and TCC restart/revocation matrix.
- Execute native system/microphone audio capture with the synthetic fixture and retain timing/accuracy evidence.
- Execute multi-monitor screenshot, keyboard-only, VoiceOver and 200% scale observations.
- Produce exact signed reference-capture evidence before enabling any generic presentation profile.
- Supply source-pinned, approved original presentation assets; arbitrary files fail closed.

T070 remains open. Automated checks and a valid bundle cannot substitute for the signed macOS, TCC, capture-client and accessibility observations required by the quickstart.
