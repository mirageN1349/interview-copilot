# Standard meeting smoke evidence

Status: **incomplete — do not mark T043 complete**

Evidence captured: 2026-08-04 (Europe/Moscow)

## Automated evidence

| Contract | Evidence |
| --- | --- |
| Meeting storage, owner isolation, two chat threads and idempotent finalization | `meeting_storage`: 3 passed |
| VAD transitions, fragment state, source-loss state and idempotent local stop | `capture_runtime`: 7 passed |
| WebSocket duplicate/replay/gap handling, question detection and separate chats | `meeting-overlay.test.ts` and WebSocket contract tests pass in the full Vitest suite |
| Local stop and immutable audit chain | Rust unit/integration suite passes |

## Signed-app path

| Required observation | Result | Gap |
| --- | --- | --- |
| Multi-speaker synthetic system/microphone audio enters ScreenCaptureKit | Not executed | Requires signed TCC-enabled audio capture and the approved audio fixture. |
| VAD creates and finalizes a playable retained recording fragment | Not executed | Requires live native sample delivery and artifact inspection. |
| Question/speaker accuracy and answer relevance | Not executed | Requires timestamped expected-vs-observed fixture results. |
| First answer appears within three seconds | Not executed | Requires end-to-end timing from captured audio through the mock stream. |
| Socket disconnect followed by overlay stop | Not executed | Domain/local-stop tests pass; signed UI behavior is not observed. |
| Selected display loss pauses capture and requires reselection | Not executed | Requires a controllable multi-display signed run. |
| Only active-profile sources appear in answers | Not executed | Mock/contract boundaries pass; retained signed meeting evidence is absent. |

T043 stays open until every signed row above is executed and linked to reproducible evidence. Automated tests are not a substitute for native audio, timing, display-loss or retained-media proof.
