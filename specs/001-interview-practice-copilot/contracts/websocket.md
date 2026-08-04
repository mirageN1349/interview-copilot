# Contract: Mock WebSocket transport

## Purpose

The mock WebSocket represents policy, transcription, question detection, answer streaming and diagram proposals. It is not an external AI service. In development/Vitest it is intercepted by MSW; packaged demo uses the same scenario reducers through an in-memory socket-compatible adapter.

## Connection

- Logical endpoint: `wss://assistant.mock.invalid/ws/v1`.
- Development uses an MSW `ws.link()` handler; no real DNS/network request is permitted.
- A future real endpoint must be explicitly added to Tauri CSP `connect-src` and pass the data-boundary gate.
- One connection exists per authenticated main-window runtime. The overlay subscribes through shared application state; it does not open a second socket.
- Client reconnect uses bounded exponential backoff with jitter: 250 ms, 500 ms, 1 s, 2 s, 4 s, then 5 s maximum; stop after 60 seconds and expose a recoverable offline state.
- Heartbeat interval is 15 seconds, timeout 5 seconds. Two missed acknowledgements trigger reconnect.

## Envelope

Every text message is UTF-8 JSON matching this envelope:

```ts
type Envelope<TType extends string, TPayload> = {
  v: 1
  id: string
  type: TType
  sentAt: string
  launchPolicyId?: string
  meetingId?: string
  correlationId?: string
  sequence?: number
  payload: TPayload
}
```

Validation rules:

- `v` must equal `1`; unknown versions are rejected with `PROTOCOL_VERSION_UNSUPPORTED`.
- `id` is a client/server-generated UUIDv7 and the idempotency key.
- `sentAt` is UTC ISO-8601 and is not trusted for ordering.
- Meeting-stream events require `meetingId`, `launchPolicyId` and a monotonically increasing `sequence`.
- Responses to commands carry the command `id` in `correlationId`.
- Unknown types or invalid payload fields are rejected and logged without rendering their raw payload.
- No binary frame or base64 media is sent in the MSW prototype. Vetted local media is referenced by artifact ID.

## Client commands

### `policy.subscribe`

```json
{
  "v": 1,
  "id": "019-command",
  "type": "policy.subscribe",
  "sentAt": "2026-08-04T10:00:00.000Z",
  "payload": { "deviceId": "managed-mac-01" }
}
```

Starts policy snapshot and kill-switch events. A mock session is required.

### `meeting.start`

Payload:

```ts
{
  profileId: string
  profileRevision: number
  captureConfigurationId: string
  mode: "standard_lab" | "adversarial_lab"
  matrixRowId?: string
}
```

Rust performs the authoritative local gate before this command may be emitted. The mock handler independently checks fixture policy and returns `meeting.accepted` or `command.error`.

### `meeting.stop`

Payload: `{ reason: "user" | "kill_switch" | "policy_lost" | "error" }`.

Idempotent. Repeated stops return the same terminal meeting result.

### `audio.fragment.ready`

Payload:

```ts
{
  artifactId: string
  startedAtMs: number
  endedAtMs: number
  source: "system" | "microphone"
  contentStatus: "allowed" | "redacted"
}
```

The artifact remains local. The scenario chooses a deterministic synthetic transcript by fixture ID; it never reads bytes through WebSocket.

### `chat.send`

Payload:

```ts
{
  thread: "live" | "side"
  content: string
  artifactIds: string[]
  contextGeneration: number
}
```

Every artifact must belong to the meeting and be `allowed` or `redacted`. A stale context generation returns `CONTEXT_GENERATION_STALE`.

### `context.reset`

Payload: `{ expectedGeneration: number }`. Success increments the generation atomically and emits `context.reset.completed`.

### `diagram.proposal.request`

Payload: `{ diagramRevision: number, questionSegmentId?: string }`. The response contains only validated graph-operation candidates.

### `heartbeat`

Payload: `{ lastReceivedSequence?: number }`.

## Server events

| Type | Required payload | Effect |
|---|---|---|
| `policy.snapshot` | Policy fields and expiry | Replace policy cache; ask Rust to re-evaluate gate |
| `kill_switch.activated` | `mode`, `reasonCode` | Immediately stop capture/model activity and audit |
| `meeting.accepted` | `meetingId`, `startedAt` | Enter running only after local Rust confirmation |
| `meeting.completed` | `meetingId`, artifact summary | Finalize query cache and route state |
| `transcript.partial` | Segment ID, text, confidence | Render ephemeral non-indexed text |
| `transcript.final` | Final segment fields | Persist/index through native command |
| `question.detected` | Segment ID, text, confidence | Auto-answer only at/above configured threshold |
| `answer.started` | Message ID, thread, source IDs | Create streaming message |
| `answer.delta` | Message ID, delta | Append only at expected sequence |
| `answer.completed` | Message ID, final metadata | Mark complete and cite profile sources |
| `diagram.patch.proposed` | Base revision, operations | Show accept/reject controls; never auto-apply |
| `context.reset.completed` | New generation | Start clean request context without deleting history |
| `command.error` | Stable code, retryable flag | Resolve correlated command as error |
| `heartbeat.ack` | Last accepted sequence | Maintain liveness |

Example final question event:

```json
{
  "v": 1,
  "id": "019-event",
  "type": "question.detected",
  "sentAt": "2026-08-04T10:01:12.000Z",
  "launchPolicyId": "019-launch-policy",
  "meetingId": "019-meeting",
  "sequence": 42,
  "payload": {
    "segmentId": "019-segment",
    "text": "Расскажите о сложном проектном решении",
    "confidence": 0.93
  }
}
```

## Ordering, duplicates and recovery

- Reducers keep `lastAppliedSequence` per meeting.
- Duplicate `id` is acknowledged but not applied again.
- An event below or equal to the applied sequence is ignored if its ID is known; an unknown old event triggers `STREAM_DIVERGED`.
- A sequence gap pauses automatic answers, requests a fixture snapshot and shows `Синхронизация…`.
- After reconnect, send `meeting.resume` with last applied sequence and context generation. Mock handler replays at most the bounded fixture window or returns a full meeting snapshot.
- `answer.delta` without `answer.started`, completion before missing deltas, or a mismatched thread is a protocol error; raw content is not rendered.
- Outbound queue is capped at 50 control messages. Audio/screenshot artifacts are not queued over this transport. Emergency stop bypasses the queue and always invokes Rust locally first.

## Error codes

| Code | Retryable | Required behavior |
|---|---:|---|
| `AUTH_SESSION_MISSING` | no | Close socket and route to sign-in after local stop |
| `POLICY_UNAVAILABLE` | yes | Block new privileged actions; retry policy only |
| `POLICY_DENIED` | no | Disable meeting/adversarial controls |
| `KILL_SWITCH_ACTIVE` | no | Stop current meeting immediately |
| `MEETING_NOT_RUNNING` | no | Reconcile local meeting state |
| `ARTIFACT_NOT_ALLOWED` | no | Remove attachment and show redaction decision |
| `CONTEXT_GENERATION_STALE` | yes | Refresh generation and require explicit resend |
| `MODEL_UNAVAILABLE` | yes | Show selected-model error; never silently switch |
| `PROTOCOL_VERSION_UNSUPPORTED` | no | Fatal transport state; require app update |
| `STREAM_DIVERGED` | yes | Request snapshot; suspend automatic answer output |

## Contract tests

- Valid/invalid envelope, unknown version/type and malformed payload.
- Duplicate commands and events remain idempotent.
- Gap, out-of-order delta, reconnect replay and snapshot recovery.
- Live and side messages never cross threads.
- Context reset prevents older generation from entering new requests.
- Kill switch stops locally even when socket send/ack is unavailable.
- Unvetted or foreign-meeting artifact is rejected.
- Model errors preserve the selected model and show explicit failure.
- MSW and packaged adapters pass the same scenario corpus.
