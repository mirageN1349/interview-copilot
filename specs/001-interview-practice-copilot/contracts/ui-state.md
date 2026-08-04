# Contract: Routes, state ownership and keyboard UI

## Routes

Vue Router uses hash history and lazy page imports.

| Route | Window | Purpose | Guard |
|---|---|---|---|
| `/sign-in` | main | Magic-link sign-in | Redirect if valid session exists |
| `/auth/verify` | main | Consume fake one-time link | Token/callback validation |
| `/permissions` | main | Live TCC checklist | Valid mock session |
| `/profiles` | main | AI profiles and source context | Session + required permissions |
| `/profiles/:profileId` | main | Vacancy, resume/manual/project context and models | Owner authorization |
| `/meetings/new` | main | Profile, model, monitor and meeting readiness | Internal run gate passes |
| `/meetings/:meetingId` | main | Active/completed meeting detail | Meeting belongs to signed-in user |
| `/history` | main | Search and retention controls | Signed-in user |
| `/account` | main | Name, email, subscription, sign-out | Session |
| `/overlay/:meetingId` | overlay only | Live/side chat and diagram | Running meeting bound to this window |

## Guard order

```text
session
  → live permission status
  → internal signed launch policy and consent
  → profile readiness
  → capture source + supported matrix row when adversarial
  → meeting route
```

- Missing session routes to `/sign-in` after local stop.
- Any required TCC permission not granted routes to `/permissions`; the route is derived from live OS state after every launch/activation.
- A denied policy or unsupported adversarial matrix row remains on readiness with stable reason codes; it never falls through to meeting start.
- The overlay route cannot be opened by typing a hash in the main window. Rust binds the overlay label to one running meeting.

## State ownership

### TanStack Query resources

- `profilesQuery` / `profileQuery(id)`
- `modelCatalogQuery`
- `meetingQuery(id)` / `meetingHistoryQuery(filters)`
- `auditQuery(filters)` for the security-admin fixture
- `subscriptionQuery`

Query keys are factories with normalized filter objects. WebSocket/native events patch or invalidate these resources; pages do not copy them into stores.

### Pinia stores

| Store | State only |
|---|---|
| `appearanceStore` | theme, OS appearance, reduce-motion/transparency/contrast signals |
| `overlayUiStore` | local visibility, position, active tab, compact/expanded state, interactive mode |
| `captureUiStore` | selected display/area draft, screenshot mode, sound-threshold draft |
| `keyboardStore` | registered bindings, conflicts, command palette state, focus return target |
| `draftStore` | unsent chat and form drafts scoped by meeting/profile |

No store owns auth session, WebSocket instance, internal launch policy, profile DTOs, message history or native permission truth.

## UI state machines

### Permission gate

```text
checking → incomplete → requesting → checking
    └────→ complete
    └────→ error → checking
```

`incomplete` selects the first required permission not granted. The app never writes a permanent `permissionsCompleted=true` flag.

### Overlay interaction

```text
hidden ↔ visible_passive ↔ visible_interactive
```

- Passive: visible, ignores pointer events, global shortcuts still work.
- Interactive: focus enters the last active control; closing interaction returns focus to the prior application where platform behavior permits.
- Emergency stop is available in all states.

### Capture

```text
idle → starting → listening → recording → listening → stopping → stopped
                └──────────→ paused/source_lost
                └──────────→ failed
```

VAD moves `listening ↔ recording`; it does not stop the underlying permission-aware stream. Indicators distinguish stream active, speech detected and artifact persisted.

### Assistant message

```text
pending → streaming → complete
   └───────────────→ error
streaming ─────────→ cancelled
```

Question confidence below threshold creates a draft question requiring confirmation; it does not enter `pending` automatically.

## Keyboard contract

All bindings are configurable. Defaults are proposals and must pass conflict registration before display as active.

| Action | Default | Scope |
|---|---|---|
| Show/hide overlay | `⌘⇧Space` | Global |
| Toggle overlay interaction | `⌘⇧O` | Global |
| Focus live chat | `⌘⇧L` | Global |
| Focus side chat | `⌘⇧J` | Global |
| Full-display screenshot | `⌘⇧S` | Global during meeting |
| Area screenshot | `⌘⇧A` | Global during meeting |
| Reset conversation context | `⌘⇧R` then explicit confirmation | Meeting |
| Emergency stop | `⌘⇧Esc` | Global during meeting |
| Move overlay | `⌃⌥Arrow` | Overlay |
| Move overlay faster | `⌃⌥⇧Arrow` | Overlay |
| Switch overlay section | `⌃Tab` / `⌃⇧Tab` | Overlay |
| Open command palette | `⌘K` | Focused window |
| Accept/reject diagram proposal | `⌘Enter` / `⌘Backspace` | Diagram |
| Select next/previous diagram node | `Tab` / `Shift+Tab` | Diagram |
| Move selected node | `Arrow`; `Shift+Arrow` faster | Diagram |
| Rename selected node | `Enter` | Diagram |
| Connect selected nodes | `C`, then choose target | Diagram |
| Undo diagram change | `⌘Z` | Diagram |

Rules:

- A conflict returned by the global-shortcut plugin leaves that action unbound and presents a remap control.
- Shortcuts are represented by action IDs so focused and global paths call the same command handler.
- Destructive/context-changing actions announce impact and require confirmation unless they are emergency stop.
- Focus is always visible in interactive mode. Opening/closing dialogs and switching chat/diagram restores a deterministic focus target.
- Pointer interactions have keyboard equivalents; drag is an enhancement, never the only path.

## Overlay information architecture

The overlay is one resizable shell with four top-level sections, not a dashboard of cards:

1. `Live` — recognized question, confidence, streaming answer and source indicators.
2. `Side` — independent user-initiated chat.
3. `Design` — semantic node list/canvas and pending patch controls.
4. `Status` — profile, model, transcription language, display, auto-screenshot, recording/capture and connection state.

Persistent controls: stop, capture state, active model/profile and compact command hint. Detailed settings remain in the main window. Build/policy diagnostics are available only through the restricted diagnostic surface.

## Visual contract

### Material hierarchy

- Native Liquid Glass/vibrancy is the outer overlay/chrome material only.
- Messages, code, diagrams and status details use solid or near-solid semantic surfaces.
- Do not nest `backdrop-filter`, glass cards or translucent dialogs inside the native material.
- Reduced Transparency or insufficient contrast switches to an opaque graphite/light surface without changing layout.

### Tokens

- Define semantic OKLCH variables for background, surface, text, muted text, border, focus, destructive, warning, success and a custom muted steel accent.
- Values are authored for this product; no default shadcn palette is accepted unchanged.
- Light/dark/auto follow system appearance. Status never relies on color alone.
- Typography uses `system-ui`; compact overlay body remains at least 13 px equivalent and critical controls scale without clipping.

### Icons and motion

- Use only named `@lucide/vue` icons in the webview, consistently at 16/18 px and one stroke width.
- Native AppKit menus may use matching SF Symbols.
- No emoji, AI-generated glyphs, decorative icon bubbles or copied third-party app identity.
- Transitions last 120–180 ms and explain state/focus only. No looping ambient animation, gradient motion or glow.

## Accessibility acceptance

- Every route and overlay action completes with keyboard only.
- VoiceOver exposes window purpose, selected chat, capture/recording state, confidence, streaming completion and diagram node/edge relationships.
- Streaming text uses a restrained live region and announces completion, not every token.
- Focus is trapped only inside modal dialogs; overlay itself does not trap focus.
- Reduced Motion removes movement/scale animation; Reduced Transparency selects the opaque material; increased contrast strengthens borders/text.
- At 200% text scaling, emergency stop, capture status and active profile remain visible without horizontal scrolling.

## UI integration tests

- Route guard table for session/permission/policy/consent/profile/matrix combinations.
- Restart at every missing permission returns to the correct live TCC step.
- Live and side drafts/history remain isolated.
- Every global action also works through focused keyboard navigation.
- Hotkey conflict is visible and remappable.
- Context reset requires confirmation and preserves visible history while changing generation.
- Low-confidence question waits for confirmation.
- Model failure never silently changes model.
- Source-lost pauses capture and asks for display selection.
- Reduce Motion/Transparency and light/dark/auto select correct tokens/material request.
- Overlay critical controls and focus order remain usable at 200% text scale.
