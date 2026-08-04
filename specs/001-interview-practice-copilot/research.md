# Phase 0 Research: Interview Copilot

**Date**: 2026-08-04
**Status**: Complete — no unresolved planning questions remain.

## R-001 — Platform and capture-exclusion feasibility

**Decision**: Target managed macOS 15+ and treat macOS 26 as the primary Liquid Glass environment. Use a normal second Tauri `WebviewWindow`. `contentProtected` is best effort only; adversarial mode is enabled per proven `macOS × conferencing client × share mode` row and fails closed everywhere else.

**Rationale**: Tauri exposes window protection, but Apple classifies `NSWindowSharingNone` as legacy and states that macOS no longer uses it to prevent screen capture. A Tauri issue demonstrates that ScreenCaptureKit-based capture ignores this protection on macOS 15+. Therefore no supported public API can guarantee that a visible local overlay is absent from every Zoom/Meet capture.

**Alternatives rejected**:

- Claiming universal invisibility: contradicted by platform behavior.
- Private window-server APIs: unsupported, brittle and inappropriate even for the test build.
- Adding `tauri-nspanel` immediately: it may improve panel focus/Spaces behavior but does not solve ScreenCaptureKit exclusion; reconsider only after the window spike.

**Sources**: [Tauri window API](https://v2.tauri.app/reference/javascript/api/namespacewindow/#setcontentprotected), [Apple `NSWindowSharingNone`](https://developer.apple.com/documentation/appkit/nswindow/sharingtype-swift.enum/none), [Tauri issue #14200](https://github.com/tauri-apps/tauri/issues/14200), [Tauri window customization](https://v2.tauri.app/learn/window-customization/).

## R-002 — Liquid Glass for Tauri

**Decision**: Use `window-vibrancy = 0.8` from the Tauri organization. Apply `NSGlassEffectViewStyle::Regular` on macOS 26+, semantic vibrancy on older macOS, and an opaque accessible surface when Reduce Transparency or contrast requirements demand it. Set `transparent: true` and `app.macOSPrivateApi: true` for the internal build.

**Rationale**: `window-vibrancy` provides `apply_liquid_glass` over AppKit `NSGlassEffectView` and a Tauri/WKWebView integration path. It is the narrowest maintained dependency for true native material. Apple positions Liquid Glass as a functional control/navigation layer, not a texture for every content card.

**Alternatives rejected**:

- `tauri-plugin-liquid-glass` and `liquid-glass-rs`: duplicate the Tauri upstream crate and broaden the native surface.
- CSS/WebGL glassmorphism libraries: simulate blur rather than native material, degrade accessibility and produce the generic AI-dashboard look explicitly rejected by the product direction.
- Tauri `windowEffects` alone: useful fallback materials, but not native Liquid Glass on macOS 26.

**Consequences**: `app.macOSPrivateApi: true` prevents Mac App Store distribution; use direct signing/notarization and managed internal distribution.

**Sources**: [`window-vibrancy` docs](https://docs.rs/crate/window-vibrancy/latest), [`window-vibrancy` releases](https://github.com/tauri-apps/window-vibrancy/releases), [Apple `NSGlassEffectView`](https://developer.apple.com/documentation/appkit/nsglasseffectview), [Apple adopting Liquid Glass](https://developer.apple.com/documentation/TechnologyOverviews/adopting-liquid-glass), [Apple Materials HIG](https://developer.apple.com/design/human-interface-guidelines/materials), [Tauri configuration](https://v2.tauri.app/reference/config/#appconfig).

## R-003 — Native screen, audio and permission APIs

**Decision**: Implement capture in a macOS-only Rust module using ScreenCaptureKit bindings. Use `SCShareableContent` and `SCContentFilter` for sources, `SCStream` for system/microphone audio and recording, and `SCScreenshotManager` plus `sourceRect` for full/area screenshots. Read real TCC status on every launch.

**Rationale**: The meeting's remote speaker is normally system audio, so microphone-only capture is insufficient. ScreenCaptureKit gives one coherent source model, supports excluding the current process from the app's own capture, and handles display-specific screenshots. TCC is the source of truth after restart; persisting a wizard index would drift from OS state.

**Permission mapping**:

- Screen recording: `CGPreflightScreenCaptureAccess` / `CGRequestScreenCaptureAccess` and ScreenCaptureKit failures.
- Microphone: `AVCaptureDevice.authorizationStatus` / `requestAccess`.
- Accessibility: `AXIsProcessTrustedWithOptions`; it is required for the narrowly scoped global event-tap/focus behavior used by keyboard area selection and cross-application overlay control. Global shortcuts alone would not justify this permission.

**Alternatives rejected**:

- Browser `getDisplayMedia`: weaker control over macOS sources and packaged behavior.
- Shelling out to `screencapture`: adds process/security complexity and cannot provide the continuous stream contract.
- Community Tauri screenshot/permissions plugins: unnecessary wrapper and version risk around APIs already needed in the Rust core.

**Sources**: [ScreenCaptureKit](https://developer.apple.com/documentation/screencapturekit), [SCScreenshotManager](https://developer.apple.com/documentation/screencapturekit/scscreenshotmanager), [SCContentFilter](https://developer.apple.com/documentation/screencapturekit/sccontentfilter/init%28display%3Aexcludingapplications%3Aexceptingwindows%3A%29), [ScreenCaptureKit audio](https://developer.apple.com/documentation/screencapturekit/scstreamconfiguration/capturesaudio), [Accessibility trust](https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions), [screen-capture preflight](https://developer.apple.com/documentation/coregraphics/cgpreflightscreencaptureaccess%28%29), [AVFoundation authorization](https://developer.apple.com/documentation/avfoundation/avcapturedevice/requestaccess%28for%3Acompletionhandler%3A%29), [`objc2-screen-capture-kit`](https://docs.rs/objc2-screen-capture-kit/latest/objc2_screen_capture_kit/).

## R-004 — Vue, Router and minimal FSD

**Decision**: Use Vue 3.5.x, Vue Router 5.x with `createWebHashHistory()`, and only FSD layers that have a present responsibility: `app`, `pages`, `features`, `entities`, `shared`; keep `mocks` as infrastructure. Lazy-load pages. Do not add `widgets` or `processes` initially.

**Rationale**: Hash history avoids server/custom-protocol fallback assumptions in a desktop SPA. FSD recommends starting with few layers and extracting features/entities only when a concept is reused. This prevents a folder-per-noun architecture before code exists.

**Boundary rules**:

- Imports flow downward: `app → pages → features → entities → shared`.
- A slice exposes a small public API at its root; internal files use relative imports.
- No global `shared/ui/index.ts`; shadcn components use per-component paths.
- Page-specific forms and panels remain inside their page.

**Sources**: [Tauri frontend setup](https://v2.tauri.app/start/frontend/), [Vue Router hash history](https://router.vuejs.org/api/functions/createwebhashhistory), [FSD layers](https://feature-sliced.design/docs/reference/layers), [FSD public API](https://feature-sliced.design/docs/reference/public-api).

## R-005 — Pinia and TanStack Query ownership

**Decision**: TanStack Vue Query owns backend-shaped resources and cache; Pinia owns synchronous local UI/runtime preferences; component `ref`s own transient local state. Do not mirror Query or auth data in Pinia.

| State | Owner |
|---|---|
| Launch Policies, profiles, vacancy/material metadata, meetings, transcripts, model catalog, entitlement | TanStack Query |
| Theme, overlay visibility/position, selected monitor, keyboard mode, local drafts | Pinia |
| Temporary form/editor interaction | Local Vue state |
| Mock auth session | Better Auth `useSession` |
| WebSocket object and reconnect state | `shared/api/ws` adapter |

WebSocket events patch or invalidate Query data; they touch Pinia only when an event changes presentation state. Every query defines an intentional `staleTime`; tests disable retry and create a fresh `QueryClient`.

**Sources**: [TanStack Query and client state](https://tanstack.com/query/latest/docs/framework/vue/guides/does-this-replace-client-state), [TanStack Query defaults](https://tanstack.com/query/latest/docs/framework/vue/guides/important-defaults), [Pinia state](https://pinia.vuejs.org/core-concepts/state.html).

## R-006 — Better Auth with an MSW-only backend

**Decision**: Use the Better Auth Vue client and magic-link client to define frontend behavior, while MSW emulates the endpoint contract. Keep the normal sign-in flow conventional and expose the generated fake link only in a developer diagnostic inbox. The build remains non-security-validating even though that implementation detail is absent from ordinary product copy.

**Rationale**: Real Better Auth magic-link authentication requires a Better Auth server plugin, delivery callback, verification-token/session storage, cookies and redirects. MSW can imitate responses but cannot turn those browser mocks into real authentication or a security boundary.

**Alternative if scope changes**: Add the smallest real Better Auth server with SQLite and mocked email delivery, leaving MSW to mock product APIs. This is explicitly outside the requested MSW-only backend.

**Sources**: [Better Auth installation](https://better-auth.com/docs/installation), [Better Auth client](https://better-auth.com/docs/concepts/client), [Magic Link plugin](https://better-auth.com/docs/plugins/magic-link), [Better Auth database model](https://better-auth.com/docs/concepts/database).

## R-007 — MSW in development, tests and packaged Tauri

**Decision**: MSW is the canonical development/test mock transport. Extract pure scenario resolvers from handlers and call them through a small in-memory adapter in packaged demo mode.

**Rationale**: Browser MSW depends on Service Worker registration over a supported secure origin. A packaged Tauri app uses platform-specific custom origins, so browser-worker registration must not be assumed. Tauri's localhost plugin could create an HTTP origin but adds a documented security risk with no benefit for a deterministic internal demo.

**Alternatives rejected**:

- Tauri localhost plugin: unnecessary expanded attack surface.
- Sidecar mock server: another process and lifecycle without a production backend requirement.
- Duplicate packaged fixtures: would drift from tested MSW scenarios.

**Sources**: [MSW browser integration](https://mswjs.io/docs/integrations/browser), [Tauri webview origins](https://v2.tauri.app/reference/javascript/api/namespacewebview/), [Tauri localhost warning](https://v2.tauri.app/plugin/localhost/).

## R-008 — WebSocket contract

**Decision**: Use the WebView's native `WebSocket`; no client library. Define a versioned discriminated JSON envelope with message/correlation IDs, sequence numbers, explicit completion/error messages, runtime validation, bounded exponential reconnect and idempotent reducers.

**Rationale**: Native WebSocket covers the transport. A small adapter isolates reconnect and parsing from stores/components. In this mock version, media remains local and messages reference vetted artifact IDs, avoiding base64 copies and unapproved egress.

**Security notes**: A future real backend should use same-origin secure cookies or a short-lived connection ticket because the browser WebSocket constructor cannot add arbitrary auth headers. Long-lived tokens must not appear in the URL. Tauri CSP restricts `connect-src`.

**Sources**: [WHATWG WebSocket](https://websockets.spec.whatwg.org/), [Tauri CSP](https://v2.tauri.app/security/csp/), [MSW WebSocket mocks](https://mswjs.io/docs/websocket/).

## R-009 — Local storage and search

**Decision**: Use the official Tauri SQL plugin with SQLite. Store metadata and text in tables, FTS5-index approved searchable content, and put media under a Rust-controlled app-data root with opaque names.

**Rationale**: Search across 10,000 meetings, relational provenance and retention are simpler and more testable in SQLite than ad hoc JSON stores. Keeping media out of rows avoids database bloat. The frontend receives IDs and DTOs, never arbitrary paths.

**Alternatives rejected**:

- Pinia/localStorage persistence for domain data: no robust search, migrations, retention or file integrity.
- A repository abstraction over every table: speculative for one storage implementation; Rust command handlers call small storage modules directly.

**Source**: [Official Tauri SQL plugin](https://v2.tauri.app/plugin/sql/).

## R-010 — Visual system, components and icons

**Decision**: Use selectively generated shadcn-vue components with custom semantic OKLCH tokens, native HTML controls where behavior fits, and named `@lucide/vue` imports. Native menus may use SF Symbols; app/alias icons are human-designed internal assets.

**Anti-slop constraints**:

- One native glass shell for overlay/chrome; no nested translucent cards.
- Graphite neutrals plus one muted steel accent; no default shadcn palette.
- No gradients, neon, glow, animated blobs, grain, decorative icon bubbles or assistant avatar.
- System font stack; do not bundle SF Pro.
- One 16/18 px outline icon family, consistent stroke width, no emoji controls.
- 120–180 ms motion, disabled/reduced according to OS accessibility preferences.
- One separator and one soft shadow, with opaque high-contrast fallback.

**Sources**: [shadcn-vue](https://github.com/unovue/shadcn-vue), [shadcn-vue theming](https://www.shadcn-vue.com/docs/theming), [`@lucide/vue`](https://www.npmjs.com/package/@lucide/vue), [Lucide upstream](https://github.com/lucide-icons/lucide), [Apple SF Symbols HIG](https://developer.apple.com/design/human-interface-guidelines/sf-symbols).

## R-011 — Code and diagram rendering

**Decision**: Lazy-load Shiki for recognized code languages and fall back to escaped plain text. Implement the first system-design canvas with semantic HTML nodes, SVG edges and validated graph operations rather than adding a graph framework.

**Rationale**: Shiki provides consistent multi-language highlighting without executing code. The requested diagram operations are bounded; direct DOM/SVG keeps keyboard semantics and AI patch validation under application control. A graph library is justified only if the feasibility implementation cannot meet navigation/edge interaction tests.

**Sources**: [Shiki documentation](https://shiki.style/), [SVG accessibility guidance](https://developer.mozilla.org/en-US/docs/Web/SVG/Guides/Accessibility).

## R-012 — Testing boundary

**Decision**: Vitest covers TypeScript behavior, components and transport contracts; `cargo test` covers native modules. Signed-app manual evidence is mandatory for TCC, global shortcuts, window level, ScreenCaptureKit, multi-monitor geometry and capture exclusion.

**Rationale**: Browser/component tests cannot prove OS permission prompts or what an external conferencing client captures. A green Vitest suite is necessary but insufficient for the defining native behavior.

**Sources**: [Vue testing recommendations](https://vuejs.org/guide/scaling-up/testing.html), [Vue Test Utils](https://test-utils.vuejs.org/installation/), [Pinia testing](https://pinia.vuejs.org/cookbook/testing.html), [Vitest request mocking with MSW](https://vitest.dev/guide/mocking/requests), [Tauri WebDriver testing](https://v2.tauri.app/develop/tests/webdriver/).

## R-013 — Transcription model catalog

**Decision**: Pin the initial selectable mock catalog entries to `openai/whisper-large-v3-turbo` and `nvidia/parakeet-tdt-0.6b-v3`. The MSW prototype stores selection and returns deterministic transcript fixtures; it does not download or run either model. Translation is a separate mocked capability and is not inferred from the ASR model name.

**Rationale**:

- The OpenAI Whisper model card identifies `whisper-large-v3-turbo` as MIT-licensed, multilingual across 99 languages, and faster than large-v3 by reducing decoder layers from 32 to 4, with a stated minor quality tradeoff.
- NVIDIA's Parakeet v3 model card identifies 25 supported European languages including Russian and Ukrainian, CC BY 4.0 terms, punctuation/timestamps and a runtime optimized for NVIDIA GPU systems with Linux as the preferred OS. It is therefore not planned as an on-device macOS dependency.
- The two model cards do not provide a comparable latency result for the target managed Mac or the future approved backend. The specification's three-second result is a mock scenario target only. Production promotion requires an approved-hardware benchmark measuring end-of-speech to final transcript/answer p50/p95, quality by language, memory and concurrent-session limits.

**Data-boundary decision**: Mock audio never leaves the local artifact store. A future catalog entry stays disabled until its exact model revision, license attribution, hosting region/runtime, processor agreement, retention/logging behavior and approved processing-boundary ID are recorded. No fallback model is selected silently.

**Sources**: [OpenAI Whisper large-v3-turbo model card](https://huggingface.co/openai/whisper-large-v3-turbo), [NVIDIA Parakeet TDT 0.6B v3 model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3).

## R-014 — Dock, app-switcher and validation alias behavior

**Decision**: Use AppKit activation policy and a small set of approved original presentation profiles. Standard mode keeps the normal application identity visible. An approved adversarial profile may switch to accessory-style Dock/app-switcher behavior and an original generic icon; every change requires the internal role, a valid signed launch policy, a supported matrix row and an audit event.

**Rationale**: AppKit is already available in the Rust boundary and can control activation policy and the application icon without another plugin. Runtime replacement cannot safely or completely change the signed bundle identity across every macOS surface. If security testing requires another bundle name, create a separately signed, managed internal alias build; never copy a real third-party bundle identifier, name or protected artwork.

**Alternatives rejected**:

- Runtime impersonation of an installed third-party application: misleading, legally risky and inconsistent across Dock, app switcher, notifications and permission prompts.
- AI-generated alias icons: inconsistent and explicitly excluded by the product direction.
- Guaranteeing invisibility in all system surfaces: platform behavior varies and remains part of the evidence matrix.

**Sources**: [Apple `NSApplication.ActivationPolicy`](https://developer.apple.com/documentation/appkit/nsapplication/activationpolicy-swift.enum), [Apple app icon HIG](https://developer.apple.com/design/human-interface-guidelines/app-icons).

## R-015 — Consumer-facing product model

**Decision**: Keep the visible application model conventional: magic-link sign-in, permissions onboarding, Home, Profiles/Context, New Meeting, live overlay, model selector, local recording, History, Settings/Subscription and Account. Internal launch policy, roles, consent records, audit and build attestation are excluded from normal navigation and user-facing copy; a denied internal gate maps to a neutral `Meeting unavailable` state with a restricted diagnostic code.

**Rationale**: The supplied `sobes.tech` reference publicly presents a simple interview-assistant flow centered on an AI assistant, multiple model providers, unified speech/screenshot/text context, local meeting recording, preparation materials and post-interview history. Those information-architecture patterns make the mock behave like a standard product. Its claims about universal invisibility, full identity mimicry and cursor concealment are not accepted as platform guarantees and its brand/assets are not copied.

**Alternatives rejected**:

- Showing internal authorization IDs, roles and audit state in the main UI: makes the replica unlike a normal product and conflicts with the requested surface model.
- Copying the reference's brand, exact layout, icons or third-party impersonation options: unnecessary for behavioral fidelity and creates legal/safety risk.
- Treating marketing claims as acceptance evidence: only the signed-app capture matrix establishes support.

**Source**: [sobes.tech public product page](https://sobes.tech/en).

## Resolved risks and promotion gates

| Risk | Planning resolution |
|---|---|
| Overlay/cursor cannot be guaranteed hidden | Supported-matrix feasibility gate; all unproven rows fail closed. |
| Better Auth requested without a real backend | Mock the Better Auth client contract and label it non-security-validating. |
| Browser MSW may not work in packaged Tauri | Reuse pure scenarios through an in-memory packaged adapter. |
| Liquid Glass conflicts with readability/accessibility | Restrict glass to overlay chrome and provide vibrancy/opaque OS-driven fallbacks. |
| Local mock policy/audit is not a production trust boundary | Synthetic-only build; real control plane and remote immutable audit are promotion requirements. |
| Accessibility permission could be over-broad | Limit it to the approved global event-tap/focus module; do not claim global shortcuts require it. |
| Retention cannot run while app is never launched | Managed daily launch is part of test operation; add a managed cleanup agent only for production promotion. |
