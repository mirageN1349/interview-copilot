# Contract: Better Auth-shaped mock authentication

## Scope and warning

This contract drives the Better Auth Vue client in development, Vitest and packaged-demo mode. MSW/in-memory handlers emulate Better Auth endpoint shapes; they do not implement a Better Auth server and must never be used to claim real authentication, email delivery or session security.

The normal UI looks like a conventional magic-link flow: email field, `Continue`, `Check your email`, verification result, and account session. Mock/build status is restricted to diagnostics and audit. Only configured synthetic identities are accepted in validation fixtures.

## Client setup

- Use `createAuthClient` from `better-auth/vue` with the magic-link client plugin.
- Route calls through an injected `fetch` transport:
  - development: browser `fetch`, intercepted by MSW;
  - Vitest: Node `fetch`, intercepted by `setupServer`;
  - packaged demo: fetch-compatible in-memory adapter invoking the same scenario resolvers.
- Better Auth `useSession` is the single frontend owner of session state. Do not copy it into Pinia.

## Endpoints

These paths are pinned as the Better Auth 1.6.x mock contract and are locked by client contract tests.

### Request magic link

`POST /api/auth/sign-in/magic-link`

```json
{
  "email": "user@example.test",
  "callbackURL": "#/auth/verify"
}
```

Success `200`:

```json
{
  "status": true
}
```

Behavior:

1. Normalize email to lowercase.
2. Reject identities outside the configured synthetic allowlist with `403 AUTH_EMAIL_NOT_ALLOWED`.
3. Create a random single-use token with a 10-minute mock expiry.
4. Store only a token hash in the scenario state.
5. Expose the fake verification URL only in the development diagnostic inbox; the normal screen remains `Check your email`.
6. Rate-limit repeated requests deterministically for the test fixture.

### Verify magic link

`GET /api/auth/magic-link/verify?token={token}&callbackURL={encodedCallback}`

Success: `302` to the allowlisted callback and a mock session established through the injected transport.

Failure responses:

| Status | Code | Meaning |
|---:|---|---|
| 400 | `AUTH_TOKEN_MISSING` | No token supplied |
| 401 | `AUTH_TOKEN_INVALID` | Hash does not match |
| 401 | `AUTH_TOKEN_EXPIRED` | More than 10 mock minutes elapsed |
| 409 | `AUTH_TOKEN_USED` | Token has already created a session |
| 400 | `AUTH_CALLBACK_INVALID` | Callback is not an internal hash route |

### Get session

`GET /api/auth/get-session`

Authenticated `200`:

```json
{
  "user": {
    "id": "019-user",
    "email": "user@example.test",
    "name": "Alex Morgan"
  },
  "session": {
    "id": "019-session",
    "expiresAt": "2026-08-04T18:00:00.000Z"
  }
}
```

Unauthenticated `200`: `null`.

The response contains no internal roles or launch-policy data. Those remain in the Rust-owned boundary.

### Sign out

`POST /api/auth/sign-out`

Success `200`:

```json
{
  "success": true
}
```

Sign-out clears scenario session state, closes WebSocket transport, asks Rust to stop any active meeting, clears Query caches containing user-scoped data and routes to `/sign-in`.

## Callback and route rules

- Allowed callbacks are internal hash routes only; default `#/auth/verify`.
- External URLs, custom-script schemes, filesystem paths and arbitrary Tauri commands are rejected.
- `/auth/verify` shows explicit used/expired/error states and never logs the raw token.
- A valid mock session does not bypass `/permissions` or the internal run gate.

## Test matrix

- Allowed and disallowed email.
- Missing, malformed, expired and reused token.
- Callback allowlist enforcement.
- Session refresh and expiry.
- Sign-out while idle and while a meeting is active.
- No raw token in application logs, audit metadata or rendered error text.
- Packaged in-memory and MSW handlers produce the same DTOs and error codes for shared fixtures.
