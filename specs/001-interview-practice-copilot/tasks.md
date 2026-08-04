---

description: "Dependency-ordered implementation tasks for Interview Copilot"
---

# Tasks: Interview Copilot

**Input**: Design documents from `/specs/001-interview-practice-copilot/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Vitest/component/contract tests and Rust tests are included because the requested stack and acceptance criteria require them. Native macOS behavior also requires signed-app evidence.

**Organization**: Tasks are grouped by user story so each story has an independently executable fixture path and an explicit validation checkpoint.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel after its stated prerequisites because it owns different files and has no incomplete dependency.
- **[Story]**: Maps the task to a user story from `spec.md`.
- Every task names the exact implementation or evidence path.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the smallest buildable Vue 3 + Tauri 2 workspace and its verification surface.

- [X] T001 Create the Vue/Vite workspace and pin only the planned frontend dependencies and scripts in `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, `tsconfig.app.json`, `vite.config.ts`, `index.html`, and `src/vite-env.d.ts`
- [X] T002 [P] Create the Tauri 2 Rust crate with the planned official plugins and macOS-only crates in `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, and `src-tauri/Info.plist`
- [X] T003 [P] Add the minimal FSD application entrypoint, hash router, lazy page placeholders, and public slice boundaries in `src/App.vue`, `src/app/entrypoint/main.ts`, `src/app/providers/index.ts`, `src/app/routes/index.ts`, and `src/app/styles/index.css`
- [X] T004 [P] Define the non-default semantic OKLCH tokens and generate only the required shadcn-vue primitives with named Lucide icons in `components.json`, `src/app/styles/tokens.css`, `src/app/styles/base.css`, `src/shared/ui/button/Button.vue`, `src/shared/ui/input/Input.vue`, and `src/shared/ui/dialog/Dialog.vue`
- [X] T005 [P] Configure ESLint, Vue TypeScript checking, Vitest, Vue Test Utils, fresh Query/Pinia test providers, and MSW test lifecycle in `eslint.config.js`, `vitest.config.ts`, `tests/setup.ts`, and `tests/helpers/render.ts`
- [X] T006 Split main/overlay capabilities, window labels, CSP, dialog/SQL/global-shortcut permissions, and private macOS webview configuration in `src-tauri/capabilities/main.json`, `src-tauri/capabilities/overlay.json`, and `src-tauri/tauri.conf.json`

**Checkpoint**: `pnpm typecheck`, an empty `pnpm test`, and `cargo check --manifest-path src-tauri/Cargo.toml` can execute against the scaffold.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish transport, storage, authorization, native command, and presentation boundaries shared by all stories.

**Critical**: No user story implementation starts until this phase passes its targeted tests.

- [X] T007 [P] Write failing common transport tests for bounded DTO parsing, stable error envelopes, scenario parity, and zero fallback network egress in `tests/contract/common-transport.test.ts` and `tests/fixtures/scenario-corpus.ts`
- [X] T008 Implement dependency-free bounded decoders, public command errors, HTTP/native gateway primitives, and request redaction in `src/shared/api/contracts/common.ts`, `src/shared/api/http.ts`, `src/shared/api/native.ts`, and `src/shared/lib/redact.ts`
- [X] T009 Implement pure scenario resolvers plus browser, Node, and packaged in-memory adapters with unhandled requests failing closed in `src/mocks/scenarios/runtime.ts`, `src/mocks/handlers/index.ts`, `src/mocks/browser.ts`, `src/mocks/node.ts`, and `src/mocks/packaged.ts`
- [X] T010 [P] Create the initial SQLite schema, FTS5 indexes, constraints, and 30-day content/365-day audit retention columns from `data-model.md` in `src-tauri/migrations/001_initial.sql`
- [X] T011 [P] Write failing Rust foundation tests for safe app-data paths, symlink escape rejection, transaction rollback, run-gate predicates, audit-chain mutation detection, and window capability checks in `src-tauri/tests/foundation_security.rs`
- [X] T012 Implement SQLite initialization, migrations, typed DTO mapping, cursor pagination helpers, and opaque app-data file storage in `src-tauri/src/storage/mod.rs`, `src-tauri/src/storage/database.rs`, `src-tauri/src/storage/files.rs`, and `src-tauri/src/storage/dto.rs`
- [X] T013 Implement policy snapshots, launch/consent/model-boundary gate predicates, append-only audit hashing, retention metadata, and fail-closed decisions in `src-tauri/src/security/mod.rs`, `src-tauri/src/security/policy.rs`, `src-tauri/src/security/run_gate.rs`, and `src-tauri/src/security/audit.rs`
- [X] T014 Implement native application state, bounded command dispatch, stable framework-error mapping, single-registration event emission, and per-window authorization in `src-tauri/src/state.rs`, `src-tauri/src/error.rs`, `src-tauri/src/events.rs`, and `src-tauri/src/commands/mod.rs`
- [X] T015 Wire QueryClient, Pinia, native-event lifecycle, shared query-key factories, and the neutral ordered route-gate pipeline in `src/app/providers/query.ts`, `src/app/providers/pinia.ts`, `src/app/providers/native-events.ts`, `src/app/routes/guards.ts`, and `src/shared/api/query-keys.ts`
- [X] T016 Implement one native material shell with macOS 26 Liquid Glass, vibrancy fallback, opaque accessibility fallback, and base main/overlay window configuration in `src-tauri/src/macos/mod.rs`, `src-tauri/src/macos/glass.rs`, `src-tauri/src/macos/windowing.rs`, and `src/app/styles/material.css`

**Checkpoint**: Common contract tests and Rust foundation tests pass; every privileged command has a native authorization point and the webview cannot address arbitrary files, SQL, or windows.

---

## Phase 3: User Story 1 - Войти и выдать разрешения (Priority: P1) — MVP

**Goal**: Complete the product-shaped magic-link flow, derive onboarding from live macOS permissions, and resume at the first missing permission after restart.

**Independent Test**: Using the allowlisted synthetic identity, consume one magic link, relaunch after each TCC decision, and reach `/profiles` only when screen recording, microphone, and Accessibility are all granted; a reused token and a revoked permission remain blocked.

### Tests for User Story 1

- [X] T017 [P] [US1] Write failing Better Auth-shaped contract tests for allowlisted email, token expiry/reuse, callback validation, session refresh, log redaction, and adapter parity in `tests/contract/auth-mock.test.ts`
- [X] T018 [P] [US1] Write failing permission restart/route-guard component tests and native TCC error-mapping tests in `tests/integration/auth-permissions.test.ts` and `src-tauri/tests/permissions.rs`

### Implementation for User Story 1

- [X] T019 [US1] Implement the shared magic-link/session scenario resolver, Better Auth Vue client with injected transport, and session cache cleanup in `src/mocks/scenarios/auth.ts`, `src/mocks/handlers/auth.ts`, and `src/shared/api/auth/client.ts`
- [X] T020 [US1] Build conventional sign-in, check-email, verification result, and development-only diagnostic inbox screens in `src/pages/sign-in/ui/SignInPage.vue`, `src/pages/sign-in/ui/CheckEmailPage.vue`, `src/pages/sign-in/ui/AuthVerifyPage.vue`, and `src/pages/sign-in/ui/DevInbox.vue`
- [X] T021 [US1] Implement live microphone, screen-recording, and scoped Accessibility status/request/settings commands using macOS APIs in `src-tauri/src/macos/permissions.rs` and `src-tauri/src/commands/permissions.rs`
- [X] T022 [US1] Implement the permission state machine, activation recheck, first-missing-step UI, and session/permission redirects without a persisted completion flag in `src/pages/permissions/ui/PermissionsPage.vue`, `src/pages/permissions/model/use-permissions.ts`, and `src/app/routes/guards.ts`
- [ ] T023 [US1] Execute the signed-app token and TCC restart matrix and record OS/build/evidence details in `tests/native-smoke/permissions.md`

**Checkpoint**: User Story 1 passes its contract, component, Rust, and signed-app permission checks independently.

---

## Phase 4: User Story 2 - Подготовить AI-профиль (Priority: P1)

**Goal**: Create isolated interview profiles from editable vacancy, resume, manual, project, and model context with visible source provenance.

**Independent Test**: Create two synthetic profiles, parse one approved vacancy URL, import resume/project fixtures, confirm extracted facts, select explicit models/language, and prove a test answer cites only the active profile.

### Tests for User Story 2

- [X] T024 [US2] Write failing model/vacancy/material HTTP contract tests and profile readiness/source-isolation integration tests in `tests/contract/product-mocks.test.ts` and `tests/integration/profile-preparation.test.ts`

### Implementation for User Story 2

- [X] T025 [P] [US2] Implement deterministic model catalog, vacancy parse, and approved-material extraction scenarios with URL allowlisting, review-required results, stable provenance, and no silent model fallback in `src/mocks/scenarios/product.ts` and `src/mocks/handlers/product.ts`
- [X] T026 [P] [US2] Implement authorized profile CRUD, revision checks, controlled fixture import, scanning/redaction status, source provenance, and archive commands in `src-tauri/src/storage/profiles.rs` and `src-tauri/src/commands/profiles.rs`
- [X] T027 [US2] Add AIProfile DTOs, TanStack Query gateways, normalized keys, and mutations without copying resource state into Pinia in `src/entities/interview-profile/model/types.ts`, `src/entities/interview-profile/api/profile-gateway.ts`, and `src/entities/interview-profile/index.ts`
- [X] T028 [P] [US2] Build profile list/create/archive navigation and empty/error states in `src/pages/profiles/ui/ProfilesPage.vue` and `src/pages/profiles/ui/ProfileCreateDialog.vue`
- [X] T029 [P] [US2] Build URL-or-text vacancy entry, editable extraction review, confirmation, and unsupported-source fallback in `src/pages/profiles/ui/VacancySection.vue`
- [X] T030 [P] [US2] Build controlled resume/project import, manual candidate context, extracted-fact review, redaction feedback, and source citation UI in `src/pages/profiles/ui/ProfileSourcesSection.vue` and `src/pages/profiles/ui/ProfileFactList.vue`
- [X] T031 [US2] Build response/transcription/translation selectors, unavailable-model errors, readiness rules, and the profile editor composition in `src/pages/profiles/ui/ModelConfigurationSection.vue`, `src/pages/profiles/model/profile-readiness.ts`, and `src/pages/profiles/ui/ProfilePage.vue`

**Checkpoint**: User Story 2 is testable with a seeded session and synthetic fixtures without requiring a running meeting.

---

## Phase 5: User Story 3 - Начать встречу (Priority: P1)

**Goal**: Start a gated meeting, capture speech automatically, show a movable keyboard-driven overlay, stream answers into isolated chats, reset context, and stop locally.

**Independent Test**: With seeded session/profile/permission fixtures, run ten synthetic questions keyboard-only, verify VAD, high/low-confidence behavior, live/side isolation, context reset, recording persistence, and local stop while the mock socket is offline.

### Tests for User Story 3

- [X] T032 [US3] Write failing WebSocket envelope/recovery/reducer tests, meeting-overlay integration tests, and Rust capture/stop transaction tests in `tests/contract/websocket.test.ts`, `tests/integration/meeting-overlay.test.ts`, and `src-tauri/tests/capture_runtime.rs`

### Implementation for User Story 3

- [X] T033 [P] [US3] Implement the versioned native WebSocket adapter, bounded reconnect/heartbeat, idempotent ordered reducers, and matching MSW/in-memory meeting scenarios in `src/shared/api/ws/protocol.ts`, `src/shared/api/ws/client.ts`, `src/mocks/scenarios/ws.ts`, and `src/mocks/handlers/ws.ts`
- [X] T034 [P] [US3] Implement meeting/profile snapshots, state transitions, two chat threads, messages, transcript persistence, and idempotent finalization in `src-tauri/src/storage/meetings.rs` and `src-tauri/src/storage/messages.rs`
- [X] T035 [P] [US3] Implement ScreenCaptureKit display/system-audio/microphone streams, process-audio exclusion, VAD thresholding, recording fragments, vetted artifact transitions before attachment/indexing, and source-lost events in `src-tauri/src/macos/capture.rs`, `src-tauri/src/macos/audio.rs`, and `src-tauri/src/storage/artifacts.rs`
- [X] T036 [US3] Implement fresh-gated `run_gate_evaluate`, transactional meeting start, idempotent stop, emergency stop, consent/boundary checks, and audit-before-success behavior in `src-tauri/src/commands/meetings.rs` and `src-tauri/src/commands/capture.rs`
- [X] T037 [US3] Implement conflict-aware official global shortcuts and the bound, movable, passive/interactive overlay window with on-screen recovery clamping in `src-tauri/src/macos/hotkeys.rs`, `src-tauri/src/macos/overlay.rs`, and `src-tauri/src/commands/overlay.rs`
- [X] T038 [US3] Add Meeting DTOs, query/native gateways, runtime event reducers, and active-meeting public API in `src/entities/meeting/model/types.ts`, `src/entities/meeting/api/meeting-gateway.ts`, `src/entities/meeting/model/runtime.ts`, and `src/entities/meeting/index.ts`
- [X] T039 [US3] Build meeting readiness with profile/model/language/display/VAD/answer-depth status and neutral blocked reasons in `src/pages/meeting/ui/NewMeetingPage.vue` and `src/pages/meeting/model/meeting-readiness.ts`
- [X] T040 [US3] Implement the meeting lifecycle action registry, capture status synchronization, keyboard conflict/remap state, and stop commands in `src/features/run-meeting/model/actions.ts`, `src/features/run-meeting/model/keyboard-store.ts`, and `src/features/run-meeting/ui/MeetingControls.vue`
- [X] T041 [US3] Build the single-shell overlay with Live, Side, Design, and Status sections, persistent recording/capture indicators, visible focus, keyboard movement, and pointer-drag enhancement in `src/features/overlay-chat/ui/OverlayShell.vue`, `src/features/overlay-chat/ui/LiveChat.vue`, `src/features/overlay-chat/ui/SideChat.vue`, and `src/features/overlay-chat/model/overlay-ui-store.ts`
- [X] T042 [US3] Implement speaker/question confidence handling, thresholded automatic answers, source citations, independent chat drafts, outbound text/artifact boundary validation, answer streaming, context-generation reset confirmation, and no cross-profile context in `src/features/overlay-chat/model/meeting-reducer.ts`, `src/features/overlay-chat/model/chat-actions.ts`, and `src/features/overlay-chat/ui/RecognizedQuestion.vue`
- [ ] T043 [US3] Execute the signed standard-meeting smoke path with multi-speaker synthetic audio, socket disconnect, display loss, recording finalization, three-second timing, speaker/question accuracy, answer relevance, and active-profile-only evidence in `tests/native-smoke/standard-meeting.md`

**Checkpoint**: The standard meeting path is independently usable end to end with synthetic fixtures; capture and stop remain native even when mock transport fails.

---

## Phase 6: User Story 4 - Работать с live coding и system design (Priority: P2)

**Goal**: Attach full/area visual context, render code safely, and edit or accept system-design graph changes entirely from the keyboard.

**Independent Test**: On a selected Retina/negative-origin display, take full and area screenshots, send a message with auto-capture, render known/unknown code, and create/move/connect/rename/delete/undo diagram nodes without a pointer.

### Tests for User Story 4

- [X] T044 [US4] Write failing screenshot geometry/redaction/attachment tests, safe code-rendering tests, and diagram revision/undo/keyboard integration tests in `src-tauri/tests/screenshot_geometry.rs`, `tests/integration/capture-context.test.ts`, and `tests/integration/system-design.test.ts`

### Implementation for User Story 4

- [X] T045 [P] [US4] Implement ScreenCaptureKit full/area screenshots, logical-point validation, Retina conversion, current-app/cursor exclusion for own artifacts, scan/redact decisions, and selected-display loss errors in `src-tauri/src/macos/screenshot.rs` and `src-tauri/src/commands/screenshots.rs`
- [X] T046 [US4] Implement monitor selection, keyboard area selection, full/area hotkeys, auto-screenshot-on-send, vetted artifact attachment, and visible redaction decisions in `src/features/capture-context/model/capture-ui-store.ts`, `src/features/capture-context/model/capture-actions.ts`, and `src/features/capture-context/ui/CaptureContextControls.vue`
- [X] T047 [P] [US4] Implement lazy Shiki rendering for allowlisted languages with escaped plain-text fallback and no code execution in `src/shared/ui/code/CodeBlock.vue` and `src/shared/lib/highlight-code.ts`
- [X] T048 [P] [US4] Implement validated diagram nodes/edges, revision checks, inverse operations, and undo without a graph dependency in `src/features/edit-diagram/model/diagram.ts` and `src/features/edit-diagram/model/operations.ts`
- [X] T049 [US4] Build semantic DOM/SVG diagram editing, keyboard commands/focus relationships, and explicit accept/reject/undo handling for WebSocket proposals in `src/features/edit-diagram/ui/DiagramEditor.vue`, `src/features/edit-diagram/ui/DiagramNode.vue`, and `src/features/edit-diagram/model/proposals.ts`
- [ ] T050 [US4] Execute the keyboard-only live-coding/system-design path across monitor change, 200% text scale, and known/unknown code fixtures and record evidence in `tests/native-smoke/live-coding-system-design.md`

**Checkpoint**: Visual context and diagrams work independently on a seeded running meeting and never put raw paths or image bytes into WebSocket frames.

---

## Phase 7: User Story 5 - Проверить поведение overlay в разных режимах захвата (Priority: P2)

**Goal**: Enable best-effort adversarial presentation only for signed, evidence-backed OS/client/share rows and fail closed on policy, matrix, consent, or kill-switch loss.

**Independent Test**: On a signed build, compare the local display, reference capture, and audit evidence for every declared matrix row; unsupported or changed rows cannot activate the mode and emergency stop completes without network acknowledgement.

### Tests for User Story 5

- [X] T051 [US5] Write failing native tests for matrix gating, best-effort guarantee reporting, presentation restoration, policy loss, kill-switch idempotence, and frontend tests forbidding restricted terms in ordinary routes/VoiceOver labels in `src-tauri/tests/adversarial_controls.rs` and `tests/integration/public-copy-boundary.test.ts`

### Implementation for User Story 5

- [ ] T052 [US5] Build and run the signed capture-feasibility probe across macOS versions, Zoom/Meet/system recording, display/window sharing, full-screen/focus/monitor topology, and Dock/menu-bar/app-switcher/notification exposure, then seed versioned evidence rows in `src-tauri/src/macos/capture_probe.rs` and `tests/native-smoke/capture-matrix.md`
- [X] T053 [US5] Persist immutable capture-matrix evidence metadata and require an exact approved row during every activation in `src-tauri/src/security/capture_matrix.rs`, `src-tauri/src/storage/capture_matrix.rs`, and `src-tauri/src/security/run_gate.rs`
- [X] T054 [P] [US5] Implement explicit best-effort content-protection state, own-capture exclusion, passive pointer behavior, keyboard-first area interaction, and guarantee-level reporting without claiming control over third-party cursor capture in `src-tauri/src/macos/overlay.rs`, `src-tauri/src/macos/screenshot.rs`, and `src-tauri/src/commands/overlay.rs`
- [ ] T055 [P] [US5] Integrate approved human-designed original assets and implement presentation profiles, AppKit activation/Dock/icon behavior, audit linkage, and unconditional restoration without third-party identity copying in `src-tauri/src/macos/presentation.rs`, `src-tauri/src/commands/presentation.rs`, `src-tauri/icons/presentation/standard.icns`, `src-tauri/icons/presentation/generic.icns`, and `src-tauri/icons/presentation/README.md`
- [X] T056 [US5] Implement policy subscription, freshness expiry, local-first stop-all, answer/capture cancellation, audit emission, and blocked recovery across socket failure in `src/shared/api/ws/policy.ts`, `src-tauri/src/security/policy.rs`, and `src-tauri/src/commands/emergency.rs`
- [X] T057 [US5] Keep ordinary readiness/overlay/notification copy consumer-shaped while routing detailed denial and evidence metadata only to the restricted diagnostic surface in `src/pages/meeting/ui/MeetingUnavailable.vue`, `src/shared/lib/public-copy.ts`, and `src/app/routes/diagnostics.ts`
- [ ] T058 [US5] Complete the signed reference-capture matrix for display/window/system capture and Dock/menu-bar/app-switcher/notification/cursor/area exposure, attach versions/checksums/results, approve only passing exact rows, and verify all missing/failing rows remain disabled in `tests/native-smoke/capture-matrix.md`

**Checkpoint**: No API-success-only claim is accepted; each enabled row has signed reference-stream evidence and every fail-safe stops locally within the required bound.

---

## Phase 8: User Story 6 - Анализировать историю и настройки (Priority: P3)

**Goal**: Search and inspect retained meetings, manage account/session, activate the Demo entitlement, and switch light/dark/auto appearance.

**Independent Test**: Seed 10,000 synthetic meetings, find one by every supported field within two seconds, inspect its retained artifacts, delete/expire content while preserving minimal audit, change theme, activate Demo idempotently, and sign out.

### Tests for User Story 6

- [X] T059 [US6] Write failing history/search/authorization/retention tests, subscription idempotence/no-payment tests, account sign-out tests, and appearance preference tests in `src-tauri/tests/history_retention.rs`, `tests/integration/history-account.test.ts`, and `tests/contract/subscription.test.ts`

### Implementation for User Story 6

- [X] T060 [P] [US6] Implement authorized title/vacancy/profile/date/transcript FTS5 search with stable cursors, meeting detail DTOs, idempotent delete/retention cleanup, audited role-gated export, and orphan verification in `src-tauri/src/storage/history.rs`, `src-tauri/src/storage/retention.rs`, and `src-tauri/src/commands/history.rs`
- [X] T061 [US6] Add meeting-history filters/query gateways and cache invalidation without Pinia duplication in `src/entities/meeting/api/history-gateway.ts`, `src/entities/meeting/model/history-filters.ts`, and `src/entities/meeting/index.ts`
- [X] T062 [US6] Build searchable history, cursor pagination, recording/transcript/chat/screenshot detail, deletion confirmation, and retention status screens in `src/pages/history/ui/HistoryPage.vue`, `src/pages/history/ui/HistoryFilters.vue`, and `src/pages/history/ui/MeetingDetailPage.vue`
- [X] T063 [P] [US6] Implement idempotent Demo entitlement scenarios and the account name/email/plan/sign-out UI with no checkout or payment identifiers in `src/mocks/scenarios/subscription.ts`, `src/mocks/handlers/subscription.ts`, and `src/pages/account/ui/AccountPage.vue`
- [X] T064 [P] [US6] Implement light/dark/auto preferences, OS appearance/accessibility signals, native material refresh, and persistent user preference storage in `src/shared/config/appearance.ts`, `src/app/providers/appearance.ts`, `src/pages/account/ui/AppearanceSettings.vue`, and `src-tauri/src/storage/preferences.rs`
- [X] T065 [US6] Seed and benchmark 10,000 meetings, prove each supported field returns within two seconds, prove expired content is inaccessible and cleaned within 24 hours while audit remains, and record query plans plus p95 evidence in `tests/performance/history-search.test.ts` and `tests/native-smoke/history-retention.md`

**Checkpoint**: User Story 6 passes with only synthetic retained data and preserves audit/storage authorization boundaries.

---

## Phase 9: Polish & Cross-Cutting Verification

**Purpose**: Prove keyboard/accessibility behavior, visual restraint, boundary hardening, and the full signed application path.

- [X] T066 [P] Add cross-story keyboard, focus restoration, VoiceOver labels, live-region restraint, Reduce Motion/Transparency, increased contrast, and 200% scaling regressions in `tests/integration/accessibility-keyboard.test.ts`
- [X] T067 [P] Polish the main and overlay surfaces to one glass hierarchy, graphite/custom-steel tokens, consistent 16/18 px Lucide icons, 120–180 ms state motion, and no gradients/glow/nested glass in `src/app/styles/tokens.css`, `src/app/styles/material.css`, and `src/features/overlay-chat/ui/OverlayShell.vue`
- [X] T068 [P] Add final no-egress, capability-denial, raw-token/path/payload redaction, audit-integrity, unsupported-matrix, and model-no-fallback regression coverage in `tests/contract/security-boundaries.test.ts` and `src-tauri/tests/security_boundaries.rs`
- [X] T069 Run `pnpm test`, `pnpm typecheck`, `pnpm lint`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `pnpm tauri build`, then record exact outputs and unresolved gaps in `specs/001-interview-practice-copilot/verification.md`
- [ ] T070 Execute every applicable step in `specs/001-interview-practice-copilot/quickstart.md` against the signed `.app`, refresh TCC and capture-matrix evidence, and record promotion blockers in `tests/native-smoke/final-validation.md`

**Checkpoint**: All automated checks pass, signed native evidence exists for every claimed macOS behavior, and any unsupported matrix row remains disabled rather than documented as complete.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 — Setup**: Starts immediately.
- **Phase 2 — Foundational**: Depends on Phase 1 and blocks all stories.
- **US1 and US2**: Start after Phase 2 and can be developed in parallel using shared synthetic fixtures.
- **US3**: Starts after Phase 2 with seeded session/profile fixtures; the full product journey composes US1 and US2.
- **US4**: Depends on the active-meeting and transport primitives from US3.
- **US5**: Depends on the overlay/capture stop path from US3; T053–T058 additionally depend on the signed feasibility result from T052.
- **US6**: Starts after Phase 2 with seeded session/meeting fixtures and can run in parallel with US2–US5.
- **Phase 9 — Polish**: Depends on every story selected for the release increment.

```text
Setup → Foundation → {US1, US2, US3, US6}
US3 → US4
US3 + signed feasibility evidence T052 → US5
{selected completed stories} → Polish
```

### Within Each User Story

1. Write the listed tests and confirm they fail for the intended missing behavior.
2. Implement native/domain primitives before pages that consume them.
3. Keep TanStack Query as resource owner and Pinia limited to local UI state.
4. Run the story's targeted automated tests.
5. Complete the stated independent test and signed-app evidence where native behavior is involved.

### Parallel Opportunities

- **Setup**: T002–T005 can run concurrently after the repository root exists; T006 follows T002.
- **Foundation**: T007, T010, and T011 are independent; T012/T013 can split between storage and security after their tests exist.
- **US1**: T017 and T018 can run concurrently; auth transport and native permissions then proceed on separate files.
- **US2**: T025 and T026 can run concurrently; T028–T030 can split after T027 exposes the profile API.
- **US3**: T033–T035 can run concurrently after T032; frontend meeting composition starts after their public contracts stabilize.
- **US4**: T045, T047, and T048 can run concurrently; T046/T049 integrate their outputs.
- **US5**: T054 and T055 can run concurrently only after T053 has an approved exact matrix row.
- **US6**: T060, T063, and T064 can run concurrently; T061/T062 follow the search contract.
- **Polish**: T066–T068 can run concurrently before the single full validation pass T069.

## Parallel Examples

```text
US1: T017 auth contracts || T018 permission/native tests
US2: T025 product scenarios || T026 native profile storage
US3: T033 WebSocket transport || T034 meeting persistence || T035 ScreenCaptureKit/VAD
US4: T045 screenshots || T047 code renderer || T048 diagram operations
US5: T054 overlay protection || T055 presentation profiles   (after T053)
US6: T060 history native core || T063 account/subscription || T064 appearance
```

## Implementation Strategy

### MVP First

1. Complete Phases 1 and 2.
2. Complete US1 (T017–T023).
3. Stop and validate the independent sign-in/permission restart journey.

This is the smallest demonstrable MVP increment. The first end-to-end interview increment additionally requires US2 and US3.

### Incremental Delivery

1. **Foundation + US1**: Session and live permission onboarding.
2. **US2**: Reusable, source-grounded profiles.
3. **US3**: Standard synthetic meeting and overlay.
4. **US4**: Screenshot, code, and diagram workflows.
5. **US5**: Evidence-gated presentation/capture behavior only where proven.
6. **US6**: History, account, Demo plan, retention, and themes.
7. **Polish**: Cross-story accessibility, hardening, build, and signed evidence.

## Notes

- `[P]` means different files and no incomplete prerequisite; it is not permission to merge conflicting edits.
- Do not add `tauri-nspanel`, window-state/process/mac-permissions plugins, a diagram framework, or another glass library unless a measured failing requirement justifies a separate plan change.
- Native TCC, global-shortcut, ScreenCaptureKit, window-level, multi-monitor, Liquid Glass, and reference-stream claims require signed-app evidence; automated web tests are not substitutes.
- Use only synthetic or explicitly consented fixture content and preserve the processing-boundary, retention, audit, and fail-closed invariants from the design artifacts.
- Commit after each task or coherent task group and rerun the nearest targeted tests before moving to the next dependency.
