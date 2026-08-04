# Implementation Plan: Interview Copilot

**Branch**: `001-interview-practice-copilot` | **Date**: 2026-08-04 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-interview-practice-copilot/spec.md` and technology constraints from [planning-inputs.md](planning-inputs.md).

## Summary

Создать macOS-приложение на Vue 3 и Tauri 2 с интерфейсом обычного interview copilot: вход по email-ссылке, запрос системных разрешений, профили и контекст, быстрый старт встречи, live/side chat, code/system-design режимы, запись, история, настройки и аккаунт. Нормальный UI не показывает внутренние роли, идентификаторы политик, аудит или терминологию тестовой среды. Vue-слой отвечает за маршруты, FSD-модули, query/UI state и два интерфейса — основное окно и keyboard-first overlay. Rust-слой владеет системными разрешениями, ScreenCaptureKit, аудио, окнами, хоткеями, локальным хранилищем и невидимым для обычной навигации fail-closed контуром запуска.

Удалённое поведение в первой версии полностью синтетическое: MSW имитирует Better Auth и WebSocket backend в разработке и Vitest, а packaged-demo использует тот же набор чистых mock-сценариев через in-memory transport. Такая сборка не является реализацией реальной аутентификации, удалённого kill switch или защищённого AI backend и не допускается к данным реальных кандидатов.

Нативный Liquid Glass применяется только к оболочке overlay на macOS 26+ через `window-vibrancy 0.8`; macOS 15 получает vibrancy/opaque fallback. Универсальная невидимость видимого overlay для Zoom, Meet и ScreenCaptureKit через публичные API недостижима, поэтому adversarial-профиль разрешается только для комбинаций `macOS × клиент × режим share`, доказанных отдельной capture-матрицей. Непроверенный сценарий блокируется.

## Technical Context

**Language/Version**: TypeScript 5.x, Vue 3.5.x, Rust stable edition 2024, Node.js 22.12+ LTS

**Primary Dependencies**: Tauri 2.11.x; Vue Router 5.x; Pinia 3.x; TanStack Vue Query 5.x; Better Auth client 1.6.x; MSW 2.x; shadcn-vue 2.8.x generated source; `@lucide/vue`; Shiki with lazy grammars; official Tauri global-shortcut and SQL plugins; `window-vibrancy = 0.8`; `objc2-screen-capture-kit = 0.3.2` and narrowly enabled macOS framework bindings

**Storage**: Local SQLite for metadata, search index and messages; application-data files for recordings/screenshots; internal append-only audit chain; macOS app-data permissions and Rust-only mutation surface. All validation fixtures are synthetic or explicitly approved.

**Testing**: Vitest 4.x, Vue Test Utils, `@pinia/testing`, MSW `setupServer`/WebSocket mocks, Rust unit/integration tests with `cargo test`, signed-app manual native smoke matrix for TCC, capture, global shortcuts and window behavior

**Target Platform**: Managed macOS 15+; macOS 26+ is the primary visual target for native Liquid Glass. Direct signed/notarized internal distribution; no Mac App Store target.

**Project Type**: Single desktop application with a Vue webview frontend and a macOS-specific Rust native core

**Performance Goals**: 95% of clear recognized questions show a complete or streaming answer within 3 seconds after question end; local overlay actions acknowledge within 100 ms p95; overlay remains responsive at 60 fps when no model stream is updating; history search completes within 2 seconds for 10,000 meetings; kill switch stops capture and suggestions within 5 seconds

**Constraints**: Keyboard-complete overlay; two isolated chats; 30-day maximum meeting retention and 365-day audit retention; no real payments; no third-party candidate data; no unapproved network boundary; no guarantee of capture exclusion outside the proven matrix; `app.macOSPrivateApi: true` prevents Mac App Store publication; MSW-only auth/control plane is mock behavior, not a production security boundary

**Scale/Scope**: One signed-in user per Mac, one active meeting, up to 10,000 indexed historical meetings, two concurrent chat streams, one selected display/area capture, recordings and screenshots retained by the active retention policy

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

The repository constitution is still an unratified placeholder and therefore defines no enforceable project principles. Until `$speckit-constitution` ratifies it, this plan uses the repository operating rules, [PRODUCT.md](../../PRODUCT.md), and the feature's security requirements as working gates.

| Working gate | Before research | After design | Evidence |
|---|---:|---:|---|
| Native-first and minimal dependencies | PASS | PASS | ScreenCaptureKit/AppKit/TCC and Tauri core are used directly; community screenshot, permission, panel and CSS-glass packages are not added. |
| Out-of-band fail-closed boundary | PASS | PASS | Meeting start is guarded in Rust by a signed launch policy, consent, role, device/environment allowlist, policy freshness and kill switch; those concepts are absent from normal product navigation. |
| Honest mock boundary | PASS | PASS | Better Auth, policy and AI responses are mocked; build attestation is available through diagnostics and audit rather than consumer-facing labels; real candidate data is rejected. |
| Clear FSD dependency direction | PASS | PASS | `app → pages → features → entities → shared`; unused layers are omitted; mocks stay outside business slices. |
| Testable contracts | PASS | PASS | WebSocket envelopes, native commands, route guards and state transitions have explicit contracts and Vitest/Rust coverage. Native OS behavior has a separate signed-app matrix. |
| Accessible, non-generic UI | PASS | PASS | Keyboard parity, VoiceOver names, visible focus, reduced motion/transparency, semantic color tokens, restrained glass and a single Lucide icon family are required. |
| No unsupported stealth claim | PASS | PASS | Capture exclusion is a feasibility gate and supported-matrix property, never a universal platform guarantee. |

## Architecture Decisions

### 1. Runtime boundaries

```text
Vue main window ─┐
                 ├─ typed Tauri commands/events ─ Rust native core ─ macOS frameworks
Vue overlay ─────┘                               ├─ SQLite + app-data files
                                                 └─ local policy/audit enforcement

Vue shared/api ─ native WebSocket/fetch ─ MSW in dev/test
                └ in-memory adapter ───── same pure mock scenarios in packaged demo
```

- Rust owns every privileged or irreversible action. Vue cannot start capture, enable adversarial behavior or write audit/storage directly.
- The main window receives only normal product configuration and authorized content commands. Internal policy/audit diagnostics stay behind Rust role checks and development/test harnesses, outside normal navigation. The overlay receives only current-meeting commands and cannot manage policies, exports or arbitrary paths.
- Remote-domain resources are represented by TanStack Query; Pinia stores only synchronous presentation/runtime state. Better Auth `useSession` owns the mocked auth session. WebSocket connection lifecycle lives in `shared/api/ws`, not in Pinia.

### 2. Native macOS implementation

| Requirement | Planned API |
|---|---|
| Display/window enumeration and capture | ScreenCaptureKit `SCShareableContent`, `SCContentFilter`, `SCStream` |
| Full/area screenshot | `SCScreenshotManager` with logical-point `sourceRect` and Retina conversion tests |
| System audio and microphone | ScreenCaptureKit audio/microphone outputs; AVFoundation fallback only if the supported OS matrix requires it |
| Screen and microphone permissions | CoreGraphics preflight/request APIs and AVFoundation authorization |
| Accessibility status and scoped input control | ApplicationServices `AXIsProcessTrustedWithOptions` for keyboard area selection/global event-tap and cross-application focus behavior; global shortcuts still use the Tauri plugin |
| Global shortcuts | Official Tauri global-shortcut plugin, including conflict reporting/remapping |
| Overlay window | Second Tauri `WebviewWindow` with transparent, frameless, always-on-top, all-workspaces configuration and keyboard position commands |
| Material | `window-vibrancy` Liquid Glass `Regular` on macOS 26; semantic vibrancy or opaque accessible fallback elsewhere |
| Capture exclusion | Tauri `contentProtected` only as best effort, validated per capture-matrix row |
| Dock/app-switcher validation profile | AppKit activation policy and approved original icon assets; separately signed internal alias bundle only when the matrix requires a different display name |

`tauri-nspanel`, `tauri-plugin-window-state`, `tauri-plugin-process`, generic screenshot libraries and third-party permissions plugins are deferred. They may be reconsidered only after a failing native spike demonstrates a concrete gap.

### 3. Mock backend and authentication

- The Better Auth Vue client and magic-link plugin shape the client API, but MSW only emulates sign-in, verification, session and sign-out. The normal UI shows the conventional `Check your email` state; a fake one-time link is exposed only in restricted developer diagnostics.
- MSW owns HTTP and WebSocket interception in Vite/Vitest. Because a packaged Tauri custom origin cannot reliably register a browser Service Worker, packaged demo mode calls the same pure scenario resolvers through an in-memory adapter.
- WebSocket messages are versioned, runtime-validated, correlated and idempotent. Large capture files remain local in this prototype; messages reference redacted local artifact IDs and receive deterministic synthetic responses.
- The initial transcription catalog contains the exact mock IDs `openai/whisper-large-v3-turbo` and `nvidia/parakeet-tdt-0.6b-v3`; neither model runs in this app. License, target runtime and approved processing boundary are catalog metadata, and translation remains a separate capability.
- A production promotion would require a real Better Auth server, secure session cookies, a real policy/control plane, approved AI/transcription processors, measured latency/quality and server-owned audit. That promotion is outside this MSW-backed plan.

### 4. Local data and retention

- SQLite tables store launch policies, profiles, meetings, transcript segments, messages, artifact metadata, model selections, preferences and append-only audit events. FTS5 indexes only fields approved for search.
- Recordings and screenshots use opaque generated filenames under the app-data directory; no user-supplied path reaches Rust file APIs.
- Every artifact passes `pending → allowed | redacted | rejected` before it may be referenced by a mock AI request. Rejected content never enters search or transport.
- Retention cleanup runs at startup and periodically while the app is running. The internal distribution policy must launch the app at least daily to satisfy the 24-hour deletion outcome; unattended enforcement outside the app would require a managed LaunchAgent and is a promotion gate, not hidden inside this prototype.
- Audit rows form an append-only hash chain and can only be created through Rust. This detects ordinary local mutation but is not presented as protection against a device administrator; production-grade immutable audit requires a remote security-owned sink.

### 5. UI architecture and visual direction

- Normal navigation is `Home`, `Profiles`, `New Meeting`, `History`, `Settings`, and `Account`. Meeting readiness presents only actionable product states such as missing permission, profile not ready, source unavailable, or meeting unavailable; internal policy IDs and roles never appear.
- The product behavior follows familiar interview-assistant patterns observed in the supplied reference: profile/context setup, one-action meeting start, model choice, unified speech/text/screenshot context, local recording and post-meeting history. It does not reuse the reference's branding, assets, identity-mimicry claims or universal invisibility claims.
- Vue Router uses `createWebHashHistory()` with lazy page modules. Route guards derive the permission route from live TCC status rather than a persisted wizard index.
- shadcn-vue is used as editable generated source, component by component. Native HTML inputs/selects are preferred where their behavior is sufficient.
- The palette uses graphite semantic neutrals plus one custom muted steel accent expressed in OKLCH; no stock shadcn color preset, purple AI gradient, glow, grain, blob or nested glass cards.
- Native glass is one structural layer around overlay chrome. Content surfaces remain opaque or lightly tinted for contrast. Light/dark/auto, Reduce Transparency, Reduce Motion and increased-contrast modes receive explicit fallbacks.
- Icons use named imports from `@lucide/vue` at consistent 16/18 px sizing. Native menus may use SF Symbols. App and validation alias icons must be human-designed original assets and must not imitate a third-party product.
- System-design mode starts with semantic DOM nodes plus an SVG edge layer and a validated graph-operation model; a graph library is not added until interaction tests prove the native implementation inadequate.
- Shiki is lazy-loaded only when a code block is present; unknown languages render as escaped plain text.

## Delivery Sequence

### Phase A — Feasibility gates

1. Build and sign the smallest Tauri app with two windows, TCC descriptions, Liquid Glass/vibrancy and best-effort `contentProtected`.
2. Record evidence for macOS 15 and 26 against the currently approved Zoom and browser-based Meet versions, for display share, window share and system screen recording.
3. Verify full-screen Spaces, focus transfer, all-workspaces behavior, global-shortcut conflicts, Retina scaling, negative multi-monitor coordinates and display reconnect.
4. Define the supported capture matrix from evidence. If no row hides the overlay/cursor/selection UI, adversarial mode remains disabled; the rest of the product can still proceed.

### Phase B — Foundation

1. Scaffold Tauri/Vite/Vue, FSD import boundaries, Router, Query, Pinia, theme tokens and two capability-scoped windows.
2. Implement shared mock scenarios, MSW browser/node adapters and packaged in-memory transport.
3. Implement a conventional magic-link UI and live permission flow. Keep mock transport/build attestation in diagnostics and audit, not in normal product copy.
4. Add Rust policy gate, SQLite migrations, audit chain and safe file roots.

### Phase C — Profiles and meeting preparation

1. Load the pre-provisioned launch policy out of band; implement only user-facing AI profiles, vacancy parsing mock, resume/manual context and project materials in the normal UI.
2. Add model/transcription/language catalog mocks, source provenance and pre-meeting readiness summary.
3. Implement monitor/area selection, hotkey remapping and meeting-start gate.
4. Add approved standard/adversarial presentation profiles for Dock, app switcher, notifications and internal alias assets; audit every transition.

### Phase D — Live meeting and overlay

1. Add ScreenCaptureKit stream, VAD-driven fragment lifecycle, local recording and screenshot artifacts.
2. Add versioned WebSocket mock stream, transcript/question/answer state machines, live and side chats, context reset and auto-screenshot behavior.
3. Implement keyboard-complete overlay movement, chat control, capture commands, Shiki rendering and system-design graph operations.
4. Apply the proven capture-matrix gate and one-action emergency stop.

### Phase E — Evidence, history and hardening

1. Add meeting finalization, FTS5 search, retention cleanup, deletion/export authorization and conventional Account/Subscription screens backed by the demo entitlement mock.
2. Complete contract, integration, accessibility and Rust tests.
3. Re-run the signed-app TCC/capture matrix and document every supported and blocked row before the adversarial profile can be enabled.

## Project Structure

### Documentation (this feature)

```text
specs/001-interview-practice-copilot/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── planning-inputs.md
├── contracts/
│   ├── auth-mock.md
│   ├── native-commands.md
│   ├── product-mocks.md
│   ├── ui-state.md
│   └── websocket.md
└── tasks.md                  # Created later by $speckit-tasks
```

### Source Code (repository root)

```text
src/
├── app/
│   ├── entrypoint/
│   ├── providers/
│   ├── routes/
│   └── styles/
├── pages/
│   ├── sign-in/
│   ├── permissions/
│   ├── profiles/
│   ├── meeting/
│   ├── history/
│   └── account/
├── entities/                 # Only reused business concepts
│   ├── interview-profile/
│   └── meeting/
├── features/                 # Only reused user actions
│   ├── capture-context/
│   ├── edit-diagram/
│   ├── overlay-chat/
│   └── run-meeting/
├── shared/
│   ├── api/
│   │   ├── auth/
│   │   ├── http/
│   │   ├── transport/
│   │   └── ws/
│   ├── config/
│   ├── lib/
│   └── ui/                   # Per-component imports; no global barrel
└── mocks/
    ├── fixtures/
    ├── handlers/
    ├── browser.ts
    ├── node.ts
    └── packaged.ts

src-tauri/
├── capabilities/
│   ├── main.json
│   └── overlay.json
├── migrations/
├── src/
│   ├── commands/
│   ├── macos/
│   │   ├── capture.rs
│   │   ├── glass.rs
│   │   ├── overlay.rs
│   │   └── permissions.rs
│   ├── security/
│   │   ├── audit.rs
│   │   └── run_gate.rs
│   ├── storage/
│   ├── lib.rs
│   └── main.rs
├── Info.plist
└── tauri.conf.json

tests/
├── contract/
├── integration/
└── fixtures/
```

Unit tests remain next to their TypeScript/Rust modules. `tests/` contains only cross-slice contracts, routed workflows and shared synthetic fixtures.

**Structure Decision**: One Tauri desktop project keeps privileged macOS behavior in a small Rust boundary and product behavior in a minimal FSD frontend. No separate backend package is created because the requested backend is MSW; mock scenarios live beside the frontend transport and are shared by development, Vitest and packaged demo adapters.

## Verification Strategy

- **Unit**: parsers, policy predicates, state transitions, query keys, graph patches, coordinate conversion, retention rules and audit hashes.
- **Component**: keyboard actions, focus restoration, route guards, query/loading/error states, two-chat isolation, code fallback and accessible status output.
- **Contract**: Better Auth mock endpoints, WebSocket envelope/payload validation, duplicate/out-of-order events, reconnect, kill switch and Tauri command errors.
- **Integration**: first launch → mock magic link → permission gate; profile → launch policy → meeting; audio/question mock → answer; screenshot redaction → chat; stop → history/search/delete.
- **Native Rust**: safe path enforcement, capability checks, permission mapping, capture configuration, monitor coordinates, artifact lifecycle and fail-closed gate.
- **Signed-app manual matrix**: TCC persistence, capture exclusion, cursor/selection visibility, full-screen Spaces, multi-monitor Retina, global shortcuts and Liquid Glass accessibility fallbacks. Vitest cannot substitute for this evidence.

## Phase Outputs

- Phase 0: [research.md](research.md)
- Phase 1 data design: [data-model.md](data-model.md)
- Phase 1 contracts: [contracts/](contracts/)
- Phase 1 validation guide: [quickstart.md](quickstart.md)
- Phase 2 task generation: `$speckit-tasks` after this plan is accepted

## Complexity Tracking

No constitution violations are recorded. The Rust native boundary and separate packaged mock adapter are required by macOS permissions/capture behavior and the absence of Service Worker support guarantees in a packaged Tauri custom origin; neither introduces a separate deployable service.
