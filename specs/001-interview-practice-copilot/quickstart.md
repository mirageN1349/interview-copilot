# Quickstart and Validation Guide

This is the Phase 1 validation contract for the planned application. The repository does not contain the application scaffold yet; the commands below become executable after `$speckit-tasks` and implementation create the paths and scripts defined in [plan.md](plan.md).

## Prerequisites

- Managed Mac with Apple silicon.
- macOS 15+ for fallback material; macOS 26+ for native Liquid Glass validation.
- Xcode Command Line Tools and a current stable Rust toolchain.
- Node.js 22.12+ and Corepack/pnpm.
- A development signing identity for reliable TCC tests; `tauri dev` alone is not sufficient evidence for permission persistence.
- Current approved Zoom build and approved browser/Meet combination for the capture matrix.
- Synthetic fixture pack only. Do not import a real candidate resume, real interview recording or non-consenting participant data.

## Planned setup commands

```bash
corepack enable
pnpm install
pnpm typecheck
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri dev
```

Expected package scripts after scaffolding:

| Script | Purpose |
|---|---|
| `pnpm dev` | Vue/Vite with browser MSW |
| `pnpm tauri dev` | Tauri application with development MSW transport |
| `pnpm test` | Vitest unit/component/contract suite once |
| `pnpm test:watch` | Local Vitest watch loop |
| `pnpm typecheck` | `vue-tsc --noEmit` |
| `pnpm lint` | ESLint over TypeScript/Vue/tests |
| `pnpm tauri build` | Signed/notarizable internal application bundle configuration |

MSW must use `onUnhandledRequest: "error"` in tests. No request to an unapproved real host is an acceptable fallback.

## Development modes

| Mode | HTTP/WebSocket behavior | Verification surface |
|---|---|---|
| Browser development | MSW Service Worker | Developer diagnostics |
| Vitest | MSW `setupServer` and `ws.link()` | Test fixture metadata |
| Tauri development | Vite HTTP origin with MSW; verify no real egress | Developer diagnostics |
| Packaged demo | Fetch/socket-compatible in-memory adapter using the same pure scenario resolvers | Signed build metadata and restricted diagnostics |

Real auth, real email, real AI/transcription, real payments and a remote policy service are absent by design.

## First functional smoke path

### 1. Mock sign-in

1. Launch with a clean app-data directory created for this test build.
2. Enter the allowlisted synthetic address `user@example.test`.
3. Confirm the normal UI says `Check your email`; obtain the fake link from the separate development diagnostic inbox.
4. Open the link once; confirm redirect to `/permissions`.
5. Reopen it; confirm `AUTH_TOKEN_USED` without exposing the token in logs.

**Pass**: Better Auth `useSession` exposes the mock identity; meeting start remains blocked until permissions and the internal gate are valid, without exposing the gate's internal labels in the normal UI.

### 2. Permission restart

Run this on a signed `.app`, not only `tauri dev`:

1. Leave all permissions undecided and relaunch; `/permissions` shows screen recording first.
2. Grant screen recording, deny microphone, relaunch; the screen-recording step is complete and microphone remains blocking.
3. Grant microphone, then grant Accessibility for the explicitly described keyboard area-selection/global event-tap and focus behavior. Verify the UI does not claim that ordinary global shortcuts require it.
4. Revoke one permission in System Settings while the app is inactive and reactivate it.

**Pass**: the route always follows current TCC state, never a persisted completion flag, and meeting creation is blocked whenever a required permission is missing.

### 3. Profile and meeting preparation

1. Create profile `Synthetic Senior Frontend`.
2. Paste an approved fixture vacancy URL; edit and confirm the extracted role and requirements.
3. Import a synthetic resume and project fixture, then add manual context.
4. Verify each answerable fact exposes its source.
5. Select a response model, `whisper-large-v3-turbo` or `parakeet`, and translation language.
6. Load the signed launch-policy fixture through the restricted setup path, then return to the normal app UI.

**Pass**: profile is ready only after sources are allowed/redacted and vacancy extraction is confirmed; normal navigation contains no internal role, policy-ID, audit or test-environment labels.

### 4. Standard meeting

1. Select a ScreenCaptureKit display and an area; inspect clamped logical coordinates/backing scale.
2. Start standard mode with a clear mock policy.
3. Play the approved synthetic interviewer audio fixture.
4. Confirm VAD changes listening → recording → listening and a final transcript appears.
5. Confirm low-confidence question waits for user confirmation; high-confidence question starts a deterministic streaming answer within the configured fixture delay.
6. Send one live and one side message; verify histories and source references stay isolated.
7. Reset context and verify visible history remains while subsequent requests use the new generation.
8. Stop from the overlay while the mock WebSocket is disconnected.

**Pass**: Rust stops locally, both chats persist, recording/artifact metadata finalizes and the meeting appears in history.

### 5. Screenshot and data-boundary behavior

1. Trigger full-display screenshot by hotkey.
2. Trigger area screenshot with a negative-origin secondary display and Retina scale.
3. Send an overlay message with auto-screenshot enabled.
4. Repeat with the fixture containing a test secret/PII marker.

**Pass**: allowed fixtures attach to the selected chat/meeting/profile; the sensitive fixture is rejected or replaced with a redacted artifact before mock AI use, with a visible decision and audit event. No raw path or bytes enter WebSocket messages.

### 6. Keyboard and accessibility

1. Disconnect the mouse/trackpad or avoid all pointer input.
2. Show, focus, move and hide the overlay; switch live/side/design/status; send messages; select display/area/model; take both screenshots; reset context; edit and connect diagram nodes; stop the meeting.
3. Repeat with VoiceOver, 200% text scaling, Reduce Motion and Reduce Transparency.

**Pass**: every core action completes from the keyboard, focus remains visible/deterministic, streaming output is announced without token spam, and opaque fallback preserves hierarchy and critical controls.

### 7. History, retention and audit

1. Seed 10,000 synthetic meetings.
2. Search by title, vacancy, profile, date and transcript text.
3. Delete one meeting and run retention for an expired fixture.
4. Verify local audit chain, then mutate a copied test database row and verify integrity failure on that isolated copy.

**Pass**: authorized results return within 2 seconds; deleted/expired content is inaccessible and absent from FTS/files; minimal audit remains; a broken chain disables privileged actions.

## Native feasibility gate

Adversarial mode must remain disabled until a signed-app evidence run produces an approved row. Create `tests/native-smoke/capture-matrix.md` during implementation with the exact app/OS versions and artifact links.

Minimum rows:

| macOS | Capture client | Mode | Overlay absent | Area UI/cursor absent | Result |
|---|---|---|---:|---:|---|
| 15.x approved patch | Zoom approved build | Share display | Record evidence | Record evidence | Supported or blocked |
| 15.x approved patch | Zoom approved build | Share window | Record evidence | Record evidence | Supported or blocked |
| 15.x approved patch | Browser + Meet approved build | Share display | Record evidence | Record evidence | Supported or blocked |
| 15.x approved patch | macOS system recording | Full display | Record evidence | Record evidence | Supported or blocked |
| 26.x approved patch | Zoom approved build | Share display | Record evidence | Record evidence | Supported or blocked |
| 26.x approved patch | Zoom approved build | Share window | Record evidence | Record evidence | Supported or blocked |
| 26.x approved patch | Browser + Meet approved build | Share display | Record evidence | Record evidence | Supported or blocked |
| 26.x approved patch | macOS system recording | Full display | Record evidence | Record evidence | Supported or blocked |

For every row also record:

- `contentProtected` requested/actual best-effort state;
- local overlay visibility and interactivity;
- full-screen Space and normal desktop behavior;
- Dock, menu bar, app switcher and notification exposure;
- selected display, Retina scale and display topology;
- whether own ScreenCaptureKit artifacts exclude the app and cursor;
- capture client version, sharing source and evidence checksum.

**Gate rule**: API success is not evidence. A row is `supported` only when the recorded reference stream satisfies the spec for that exact combination. Every missing, changed or failing row is blocked by the Rust run gate.

## Failure-path matrix

| Fault | Expected result |
|---|---|
| Policy fixture unavailable/expired | New meeting and privileged actions blocked; active meeting stops |
| Kill switch `stop_all` | Capture and answers stop locally within 5 seconds, even if acknowledgement fails |
| Selected display disconnected | Capture pauses; explicit reselection required; no silent monitor change |
| Hotkey collision | Binding remains inactive and UI requests remap |
| Selected model unavailable | Explicit error; selected model is not silently replaced |
| WebSocket duplicate/out-of-order/gap | Idempotent ignore or snapshot recovery; automatic answers pause on divergence |
| Audit append fails | Privileged transaction rolls back |
| Artifact scan rejects content | No chat attachment, index entry or transport reference |
| Overlay position becomes off-screen | Position clamps recovery handle into an available display |
| Permission revoked while running | Stream stops and app returns to permission gate |

## Verification commands before completion claims

```bash
pnpm test
pnpm typecheck
pnpm lint
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

Then run the signed `.app` permission flow and every capture-matrix row affected by the change. A passing build or Vitest suite is not sufficient evidence for TCC, capture exclusion, Liquid Glass, global shortcuts or multi-monitor behavior.

## Promotion blockers

Do not use real candidate data or ship outside the authorized test distribution until all of the following exist outside this prototype:

- real Better Auth server and approved email delivery;
- remote, security-owned policy/kill-switch and immutable audit services;
- approved AI/transcription processors and documented data boundaries;
- unattended retention enforcement for devices that may not launch the app daily;
- renewed capture-matrix evidence for every supported OS/client update;
- security and privacy approval for distribution, consent and export workflows.
