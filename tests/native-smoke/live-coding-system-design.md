# Live coding and system-design smoke evidence

Status: **incomplete — do not mark T050 complete**

Evidence captured: 2026-08-04 (Europe/Moscow)

## Automated evidence

| Check | Command | Result |
| --- | --- | --- |
| Retina/negative-origin geometry, bounds, redaction and artifact state | `cargo test --manifest-path src-tauri/Cargo.toml --test screenshot_geometry` | 6 passed |
| Known/unknown code fixtures and safe fallback | `pnpm vitest run tests/integration/code-rendering.test.ts` | 4 passed |
| Diagram revision, undo, proposal and keyboard operations | `pnpm vitest run tests/integration/system-design.test.ts` | 5 passed |
| Monitor/area state and vetted attachment | `pnpm vitest run tests/integration/capture-context.test.ts` | 3 passed |

The native screenshot command excludes the current application, disables cursor capture for its own artifact, converts global logical coordinates to display-local source coordinates, and exposes only an approved/redacted artifact ID to chat.

## Signed-app matrix

| Required path | Result | Remaining evidence |
| --- | --- | --- |
| Select a monitor, then disconnect or change it | Not executed | Requires a signed application with a controllable multi-display setup. |
| Capture a full display and a keyboard-defined area | Not executed | Requires interactive signed-app ScreenCaptureKit/TCC evidence. |
| Send a side question with automatic screenshot context | Not executed | Requires observing the signed overlay and retained artifact together. |
| Render allowlisted and unknown code at 200% text scale | Not executed | Requires visual inspection for clipping, focus visibility and readable fallback. |
| Create, move, connect, rename, delete and undo diagram nodes without a pointer | Not executed | Automated DOM coverage passes, but signed keyboard/VoiceOver behavior is not yet observed. |

## Completion gap

T050 remains open until the signed matrix above is executed with timestamped screenshots or an equivalent reproducible recording. Automated tests prove domain and DOM behavior, but not macOS focus, TCC, multi-monitor or 200% visual behavior.
