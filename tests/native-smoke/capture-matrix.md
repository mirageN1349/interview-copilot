# Signed capture matrix

Status: **BLOCKED — no signed reference-capture evidence has been produced.**

An automated test, successful `contentProtected` call, or local screenshot is not approval evidence. Every row below remains disabled until a signed `.app` run records immutable artifact checksums, exact versions, and the reference stream result.

Build available for the pending matrix: Apple Development signed bundle, executable SHA-256 `11cb4ea039c7886467c3290ae36d0e1fdee960ac4158ad2d04ba90728eb5ec98`. Strict signature verification passes; this alone does not approve a row.

| Row | macOS | Capture client | Share mode | Topology / state | Overlay absent | Area UI and cursor absent | System surfaces checked | App signature | App checksum | Evidence checksum | Result |
|---|---|---|---|---|---:|---:|---|---|---|---|---|
| mac15-zoom-display | 15.x exact patch pending | Zoom exact build pending | Display | Retina, windowed/full-screen pending | Not run | Not run | Dock/menu/app switcher/notifications not run | Not run | Missing | Missing | BLOCKED |
| mac15-zoom-window | 15.x exact patch pending | Zoom exact build pending | Window | Retina, windowed/full-screen pending | Not run | Not run | Dock/menu/app switcher/notifications not run | Not run | Missing | Missing | BLOCKED |
| mac15-meet-display | 15.x exact patch pending | Browser + Meet exact builds pending | Display | Retina, windowed/full-screen pending | Not run | Not run | Dock/menu/app switcher/notifications not run | Not run | Missing | Missing | BLOCKED |
| mac15-system-recording | 15.x exact patch pending | macOS recording exact build pending | Display | Retina, windowed/full-screen pending | Not run | Not run | Dock/menu/app switcher/notifications not run | Not run | Missing | Missing | BLOCKED |
| mac26-zoom-display | 26.x exact patch pending | Zoom exact build pending | Display | Retina, windowed/full-screen pending | Not run | Not run | Dock/menu/app switcher/notifications not run | Not run | Missing | Missing | BLOCKED |
| mac26-zoom-window | 26.x exact patch pending | Zoom exact build pending | Window | Retina, windowed/full-screen pending | Not run | Not run | Dock/menu/app switcher/notifications not run | Not run | Missing | Missing | BLOCKED |
| mac26-meet-display | 26.x exact patch pending | Browser + Meet exact builds pending | Display | Retina, windowed/full-screen pending | Not run | Not run | Dock/menu/app switcher/notifications not run | Not run | Missing | Missing | BLOCKED |
| mac26-system-recording | 26.x exact patch pending | macOS recording exact build pending | Display | Retina, windowed/full-screen pending | Not run | Not run | Dock/menu/app switcher/notifications not run | Not run | Missing | Missing | BLOCKED |

For every future evidence row record:

- signed bundle identity and verification output;
- app, OS, capture client, browser, and meeting client exact versions;
- display/window source, full-screen/focus state, monitor topology, and Retina scale;
- local overlay state and `best_effort` content-protection state;
- own ScreenCaptureKit exclusion result separately from third-party cursor behavior;
- Dock, menu bar, app switcher, notification, pointer, and keyboard area-selection exposure;
- SHA-256 checksums for the app build and immutable reference capture.

Approval rule: only a passing, signed, exact row may be inserted as `approved`. Any missing field, version drift, failed surface, or checksum mismatch is `blocked`.
