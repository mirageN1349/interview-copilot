# Contract: Consumer-facing MSW product mocks

## Scope

These HTTP mocks cover remote-shaped product operations that are not native macOS responsibilities: model catalog, vacancy extraction, profile-material extraction and demo subscription state. The normal UI uses conventional product responses and copy. MSW handles development/Vitest requests; packaged demo invokes the same pure scenario resolvers through the in-memory adapter.

Profiles, meetings, recordings, screenshots, transcripts and history remain canonical in Rust/SQLite and are accessed through native commands wrapped by TanStack Query. MSW must not create a second persistent copy of those resources.

## Common format

Success:

```ts
type Success<T> = {
  data: T
  requestId: string
}
```

Failure:

```ts
type ApiError = {
  error: {
    code: string
    message: string
    retryable: boolean
    field?: string
  }
  requestId: string
}
```

- JSON is UTF-8; timestamps are UTC ISO-8601.
- Unknown fields are ignored in responses but rejected in mutation requests.
- Scenario latency is deterministic per fixture and can be overridden by a test.
- `requestId` is safe for user-visible support copy; internal policy/audit identifiers are never returned.
- Unhandled requests fail in development/test and never fall through to a real host.

## Model catalog

### `GET /api/models`

Query: optional `kind=response|transcription|translation`.

```json
{
  "data": [
    {
      "id": "openai/whisper-large-v3-turbo",
      "kind": "transcription",
      "name": "Whisper Large v3 Turbo",
      "description": "Multilingual transcription",
      "availability": "available",
      "languages": ["auto", "en", "ru"],
      "capabilities": ["streaming", "timestamps"]
    },
    {
      "id": "nvidia/parakeet-tdt-0.6b-v3",
      "kind": "transcription",
      "name": "Parakeet TDT 0.6B v3",
      "description": "Multilingual transcription with punctuation",
      "availability": "available",
      "languages": ["en", "ru", "uk"],
      "capabilities": ["timestamps", "punctuation"]
    }
  ],
  "requestId": "req-models-1"
}
```

The UI displays only user-relevant fields. License, benchmark and processing-boundary metadata stay in the internal catalog record. Selecting an unavailable model returns an explicit error and never silently changes the choice.

## Vacancy extraction

### `POST /api/vacancies/parse`

Request accepts exactly one source:

```ts
{
  source: { kind: "url"; url: string } | { kind: "text"; text: string }
}
```

Success `200`:

```json
{
  "data": {
    "title": "Senior Frontend Engineer",
    "company": "Example Product",
    "responsibilities": ["Build product interfaces"],
    "requirements": ["Vue 3", "TypeScript"],
    "summary": "Frontend role in a product team",
    "confidence": 0.91,
    "needsReview": true,
    "sourceLabel": "Fixture vacancy"
  },
  "requestId": "req-vacancy-1"
}
```

Rules:

- URL fixtures are allowlisted and resolved locally; the mock never fetches an arbitrary site.
- Text is size-bounded and scanned before handler invocation.
- Every extraction is editable and remains `needsReview=true` until the user confirms it.
- Unsupported/private URL returns `VACANCY_SOURCE_UNAVAILABLE` with a suggestion to paste text.

## Profile-material extraction

### `POST /api/profile-materials/extract`

Request:

```ts
{
  sourceId: string
  kind: "resume" | "project"
  contentStatus: "allowed" | "redacted"
}
```

The handler receives an approved source ID, never a local path or raw file bytes. It returns editable facts with stable source ranges:

```json
{
  "data": {
    "facts": [
      {
        "id": "fact-1",
        "category": "project_result",
        "text": "Reduced page load time by 30%",
        "sourceRange": "page:1"
      }
    ],
    "needsReview": true
  },
  "requestId": "req-material-1"
}
```

Foreign, pending or rejected source IDs return `PROFILE_SOURCE_NOT_ALLOWED`.

## Demo subscription

### `GET /api/subscription`

Returns a product-shaped plan summary:

```json
{
  "data": {
    "plan": "demo",
    "status": "inactive",
    "features": ["Live assistant", "Profiles", "Meeting history"],
    "expiresAt": null
  },
  "requestId": "req-subscription-1"
}
```

### `POST /api/subscription/activate`

Request: `{ "plan": "demo" }`.

Success activates the demo entitlement immediately without card details, checkout redirect, payment identifier or network call. The Account screen uses ordinary plan UI and labels the plan `Demo`; audit/diagnostics retain `source=mock_button`.

Repeated activation is idempotent and returns the existing entitlement.

## Profile and meeting native query facade

TanStack Query functions expose a consistent product-level API even though the implementation calls Tauri commands:

| Query/mutation | Native command |
|---|---|
| `profilesQuery` | `profile_list` |
| `profileQuery(id)` | `profile_get` |
| create/update profile | `profile_save` |
| archive profile | `profile_archive` |
| `meetingHistoryQuery(filters)` | `meeting_search` |
| `meetingQuery(id)` | `meeting_get` |
| delete meeting | `meeting_delete_content` |

Components do not know whether a query function uses HTTP, WebSocket or Tauri invoke. This is transport isolation, not a repository framework: each query calls one small gateway function.

## Contract tests

- Normal response/error shape and stable request IDs.
- Model filter, unavailable model and no silent fallback.
- Vacancy URL/text exclusivity, size bound, fixture allowlist and editable review state.
- Material source ownership/status and no path/bytes in request.
- Demo activation contains no payment fields or outbound request and is idempotent.
- MSW and packaged adapter return identical DTOs for the shared scenario corpus.
- Native query facade preserves Query cache ownership without copying data into Pinia.
