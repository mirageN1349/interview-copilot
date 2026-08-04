# Signed-app authentication and permission smoke evidence

Status: **incomplete — do not mark T023 complete**

Evidence captured: 2026-08-04 (Europe/Moscow)

## Build under test

| Field | Observed value |
| --- | --- |
| Host OS | macOS 27.0 (26A5388g) |
| App | `src-tauri/target/release/bundle/macos/Interview Copilot.app` |
| Bundle identifier | `com.interviewcopilot.desktop` |
| Version / build | `0.1.0` / `0.1.0` |
| Architecture | arm64 |
| Signing authority | Apple Development: Ivan Samarin (92FRRZSP3U) |
| Team identifier | `4NT53972NZ` |
| Executable SHA-256 | `11cb4ea039c7886467c3290ae36d0e1fdee960ac4158ad2d04ba90728eb5ec98` |

## Verified evidence

- `codesign --verify --deep --strict` reported that the bundle is valid on disk and satisfies its designated requirement.
- The current signed bundle passes strict on-disk verification. A previous signed build and the current debug build launched successfully; the current release artifact has not received a fresh interactive TCC run.
- `Info.plist` contains microphone and screen-capture usage descriptions.
- Contract tests cover token expiry/reuse and component/Rust tests cover route ordering and permission mapping, but those automated tests are not signed-app TCC evidence.

## Signed-app matrix

| Check required by quickstart | Result | Evidence / gap |
| --- | --- | --- |
| Consume the allowlisted synthetic magic link once | Not executed | The diagnostic inbox is compiled behind `import.meta.env.DEV`; the packaged signed build provides no restricted way to retrieve its generated link. |
| Reopen the same link and observe `AUTH_TOKEN_USED` without token leakage | Not executed | Blocked by the same packaged-link retrieval gap; signed-app logs were not exercised with a known token. |
| Relaunch with all permissions undecided and show screen recording first | Not executed | Requires a controlled TCC reset plus interactive UI observation. |
| Grant screen recording, deny microphone, relaunch, and keep microphone blocking | Not executed | Requires interaction with the signed app and System Settings, followed by route observation after relaunch. |
| Grant microphone and Accessibility and reach the next product route | Not executed | Requires interaction with the signed app, macOS prompts/System Settings, and route observation. |
| Revoke one permission while inactive, reactivate, and return to the permission gate | Not executed | Requires live TCC mutation and foreground-activation observation. |
| Confirm meeting creation remains blocked whenever a required permission is missing | Not executed in the signed app | Covered only by automated route-gate tests, which cannot prove current TCC state or packaged-app behavior. |

## Remaining completion gap

T023 remains blocked on a real signed-app UI/TCC run. Complete it only after a signed test build exposes a restricted diagnostic method to obtain the generated mock magic link (without logging the token), and an operator or working macOS UI automation executes every row above with screenshots or timestamped observations. A valid signature, successful launch, Vitest, and Rust tests are necessary but do not satisfy the native acceptance criterion.
