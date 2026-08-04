# Contract: Tauri native commands and events

## Boundary rules

- Every privileged operation is implemented in Rust and exposed as a narrow Tauri command.
- Inputs are deserialized into bounded structs; unknown fields, invalid enum values, NaN/infinite coordinates, oversized strings and arbitrary filesystem paths are rejected.
- Rust derives the active user, device and meeting from native state. The caller cannot authorize itself by passing a role or email.
- Meeting/capture commands perform a fresh local run-gate check and append an audit event before returning success.
- Commands return `Result<T, CommandError>`; no panic text, local path or raw framework error crosses the webview boundary.
- The overlay window has a smaller Tauri capability file than the main window.

## Shared DTOs

```ts
type CommandError = {
  code: string
  message: string
  retryable: boolean
  recovery?: "open_settings" | "reselect_display" | "rebind_hotkey" | "restart_app"
}

type PermissionState =
  | "not_determined"
  | "granted"
  | "denied"
  | "restricted"

type PermissionSnapshot = {
  screenRecording: PermissionState
  microphone: PermissionState
  accessibility: PermissionState
  observedAt: string
  restartMayBeRequired: boolean
}

type DisplayDescriptor = {
  displayId: number
  label: string
  frame: { x: number; y: number; width: number; height: number }
  backingScale: number
  isPrimary: boolean
}
```

Display labels are UI hints only. Capture always identifies a source by ScreenCaptureKit `displayId`.

## Permission commands

| Command | Window | Input | Output | Notes |
|---|---|---|---|---|
| `permissions_status` | main | none | `PermissionSnapshot` | Reads live TCC state; never trusts persisted wizard state |
| `permissions_request` | main | `{ kind }` | `PermissionSnapshot` | `kind` is screen, microphone or accessibility; may only initiate the OS flow. Accessibility supports the scoped global event-tap/focus module, not global shortcut registration. |
| `permissions_open_settings` | main | `{ kind }` | `void` | Opens the matching System Settings pane when direct prompting is unavailable |

After any app activation event while the permissions page is visible, the UI calls `permissions_status` again. A denied permission never becomes granted merely because the request command returned.

## Policy and meeting commands

### `run_gate_evaluate`

Input:

```ts
{
  launchPolicyId: string
  requestedMode: "standard_lab" | "adversarial_lab"
  captureConfigurationId?: string
}
```

Output:

```ts
{
  allowed: boolean
  reasonCodes: string[]
  policyVersion?: string
  policyExpiresAt?: string
  matrixRowId?: string
}
```

Required checks: mock session binding, user status, device/environment allowlist, approved/unexpired launch policy, participant consent, requested capture scopes, policy freshness, kill switch, processing boundary and adversarial role/approval/matrix row.

### `meeting_start`

Main-window only. Input contains `launchPolicyId`, `profileId`, `profileRevision`, `captureConfigurationId` and mode. Rust repeats the gate, snapshots configuration, creates both chat threads and recording state, then returns a `MeetingRuntimeSummary`. It must not start capture if audit append or database transaction fails.

### `meeting_stop`

Main and overlay. Input `{ meetingId, reason }`. Idempotently stops ScreenCaptureKit streams, blocks new artifacts/AI operations, flushes complete fragments, finalizes storage and emits `meeting://state`. Local stop completes even if mock WebSocket is unavailable.

### `emergency_stop_all`

Main and overlay. No input. Highest-priority local action: stop all streams and pending screenshots, make overlay non-interactive, cancel mock answer streams, mark meeting stopping, append audit, then attempt mock policy notification. It is safe to invoke repeatedly.

## Display and capture commands

| Command | Window | Input | Output |
|---|---|---|---|
| `displays_list` | main, overlay | none | `DisplayDescriptor[]` |
| `capture_validate_area` | main, overlay | `{ displayId, area }` | Clamped area plus backing scale |
| `capture_start` | main | `{ meetingId }` | Capture runtime status |
| `capture_pause` | main, overlay | `{ meetingId, source }` | Capture runtime status |
| `capture_screenshot` | main, overlay | `{ meetingId, displayId, area?, chatThread }` | Vetted artifact summary or redaction error |
| `capture_stop` | main, overlay | `{ meetingId, reason }` | Capture runtime status |

Rules:

- `capture_start` reads the meeting's stored configuration; the webview cannot substitute an arbitrary source.
- Full/area screenshots use ScreenCaptureKit. `area` is logical points and is converted using the selected display's current scale.
- Area coordinates must remain inside the display frame. Negative global coordinates are allowed before conversion but size must be positive and bounded.
- `showsCursor` is false for the app's own artifact. This does not claim control over the cursor captured by Zoom/Meet.
- The current application is excluded from its own ScreenCaptureKit filter where supported.
- Every raw artifact starts `pending`; secret/PII scanning produces `allowed`, `redacted` or `rejected` before the command returns an attachable ID.
- A display disconnect emits `capture://source-lost`, pauses capture and requires reselection; it never silently switches monitor.

## Overlay and hotkey commands

| Command | Window | Input | Output |
|---|---|---|---|
| `overlay_open` | main | `{ meetingId }` | Overlay runtime summary |
| `overlay_show` / `overlay_hide` | main, overlay | `{ meetingId }` | Visibility state |
| `overlay_move` | overlay | `{ meetingId, dx, dy }` | Clamped position |
| `overlay_set_position` | main, overlay | `{ meetingId, displayId, x, y }` | Clamped position |
| `overlay_set_interactive` | overlay | `{ interactive }` | Interaction state |
| `overlay_apply_material` | main | `{ appearance }` | `liquid_glass`, `vibrancy` or `opaque` |
| `overlay_set_capture_protection` | main | `{ meetingId, enabled, matrixRowId }` | Best-effort state and explicit guarantee level |
| `hotkeys_register` | main | Action-to-accelerator map | Per-action success/conflict results |
| `hotkeys_unregister_all` | main | none | `void` |
| `presentation_profiles_list` | main | none | Approved standard/adversarial internal profiles |
| `presentation_profile_apply` | main | `{ meetingId, profileId }` | Actual activation/Dock/icon state plus audit ID |

`overlay_set_capture_protection` output includes `guarantee: "best_effort"`; the API must never return `guaranteed`. It refuses adversarial enablement without an approved matrix row even if the OS call itself succeeds.

The passive overlay calls the Tauri ignore-cursor-events behavior. Keyboard movement works in fixed logical-point increments and clamps a recovery handle inside a visible display.

`presentation_profile_apply` repeats the adversarial gate, accepts only security-owned internal assets and always restores the standard identity when the meeting stops. A different signed bundle name is a build-time managed variant, not arbitrary runtime input.

## Storage and audit commands

| Command | Window | Purpose |
|---|---|---|
| `profile_list` | main | Return profile summaries for the signed-in user |
| `profile_get` | main | Return one authorized profile and source provenance |
| `profile_save` | main | Create/update a validated profile revision |
| `profile_archive` | main | Archive a profile not used by an active meeting |
| `profile_source_import` | main | Copy an allowlisted fixture file into controlled storage, scan it and create provenance |
| `meeting_search` | main | Authorized FTS search with cursor pagination |
| `meeting_get` | main | Return authorized meeting DTO without internal storage keys |
| `meeting_delete_content` | main | Execute idempotent content deletion and preserve minimal audit |
| `meeting_export` | main | Security/exporter role only; create an audited bounded bundle |
| `retention_run` | main/system | Expire and remove due content |
| `audit_verify_chain` | main/system | Verify local append-only hash chain |
| `audit_query` | main | Security-admin fixture only; metadata, never raw media |

File-selection uses the official Tauri dialog capability in the main window. The dialog result is passed once to Rust import logic; the path is not stored or returned to the overlay.

## Native events

| Event | Payload | Consumer behavior |
|---|---|---|
| `permissions://changed` | `PermissionSnapshot` | Re-run route guard/readiness |
| `meeting://state` | Meeting runtime summary | Patch Query and overlay controls |
| `capture://state` | Source/VAD/recording status | Show persistent local indicator |
| `capture://source-lost` | Display ID and reason | Pause and require explicit reselection |
| `artifact://ready` | Public artifact summary | Permit attachment/indexing |
| `artifact://rejected` | Artifact ID and redaction reason | Show decision; do not expose bytes |
| `hotkey://action` | Action ID | Dispatch through the same action registry as focused shortcuts |
| `policy://stale` | Reason code | Fail closed and stop active privileged work |
| `audit://integrity-failed` | Sequence and reason | Disable privileged actions and show critical status |

Listeners are registered once by the app provider and disposed on teardown. Pages/components do not register duplicate global listeners.

## Capability split

### Main window

Allowed: auth transport, permission commands, profile CRUD, display selection, meeting start/stop, file dialog/import, search/history, export when role permits, theme/material configuration and hotkey registration. Launch-policy state is Rust-owned and exposed only as a neutral readiness result.

Denied: shell execution, arbitrary filesystem access, unrestricted URL opening, arbitrary SQL, hidden plugin commands and development mock inbox in production-labeled builds.

### Overlay window

Allowed: current-meeting read model, show/hide/move, interaction toggle, chat send, approved screenshot, context reset, diagram operations, meeting stop/emergency stop.

Denied: auth mutation, profile editing, allowlist/policy mutation, arbitrary meeting IDs, file dialogs, exports, raw recording access, SQL and navigation outside the overlay route.

## Stable error codes

| Code | Meaning |
|---|---|
| `PERMISSION_REQUIRED` | Required TCC permission is not granted |
| `RUN_GATE_DENIED` | One or more policy/launch policy checks failed |
| `POLICY_STALE` | Snapshot expired or unavailable |
| `KILL_SWITCH_ACTIVE` | New/active work is blocked |
| `ADVERSARIAL_MATRIX_UNSUPPORTED` | No proven capture-matrix row |
| `DISPLAY_NOT_FOUND` | Selected ScreenCaptureKit display disappeared |
| `CAPTURE_START_FAILED` | Framework rejected stream configuration |
| `AREA_OUT_OF_BOUNDS` | Invalid screenshot rectangle |
| `HOTKEY_CONFLICT` | Accelerator is already registered/reserved |
| `ARTIFACT_REJECTED` | Secret/PII boundary check denied content |
| `STORAGE_PATH_DENIED` | Path escaped controlled root or was caller-supplied |
| `AUDIT_INTEGRITY_FAILED` | Hash chain cannot be verified |
| `WINDOW_CAPABILITY_DENIED` | Calling window lacks command capability |

## Required Rust tests

- Gate predicate table for every missing/expired/denied condition.
- Overlay command rejected from the wrong window or wrong meeting.
- Safe path join and symlink escape attempts.
- Area conversion across Retina scale and negative secondary-display origins.
- Meeting start transaction rollback when audit/storage/capture setup fails.
- Emergency stop idempotence and completion without WebSocket.
- Audit chain append/verify and mutation detection.
- Retention deletion idempotence and search-index removal.
- Stable mapping from macOS framework errors to public error codes.
