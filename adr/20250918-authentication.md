# Users authentication strategy

_Date:_ 2025-09-18, updated 2026-08-25

## Context

The _multi-user_ version of the _activities_ needs to support user
authentication. We want to keep the authentication flow simple to setup in order
to make self-hosting the application easy, while maintaining a good user
experience.

We chose a password-less flow that sends short-lived single-use authentication
link by email on login. When an authentication link is reached, a session token
is created and written to the user's browser's cookies. On subsequent requests
the server extracts the session cookie and matches it to the list of known
active sessions to retrieve user's information like its ID. Those information
are then passed to the handler that will process the incoming request.

Session tokens are opaque and contain no user information. The session lifecycle
is fully managed on the server side.

## Security Considerations

- Authentication links expire after 15 minutes and are single-use,
- Session tokens are cryptographically random (192-bit), and are stored as hash,
- Sessions expire after 30 days of inactivity,
- Email is assumed to be a reasonably secure channel for our threat model.

### Session token hashing algorithm (added 2026-08-25)

Session tokens were initially hashed with Argon2, the same algorithm one would
use for user passwords. This turned out to be the wrong tool as Argon2's
deliberate slowness exists to protect low-entropy secrets (passwords) against
offline brute force. Our session tokens already carry 192 bits of entropy so
brute-forcing a leaked token hash is considered infeasible at that entropy
regardless of hash speed. Thus Argon2 bought no additional security while adding
significant compute time to every request (plus it scaled linearly with the
number of session).

Session tokens are now hashed with plain SHA-256. We also considered keying the
hash (HMAC-SHA256) for extra defense-in-depth against a database-only leak, but
decided against it: on a single-server deployment, a leak of the session store
and a leak of an HMAC key are likely to happen together (e.g. a full disk or
backup compromise), which removes most of HMAC's benefit. On the other hand a
dedicated key adds real operational cost (provisioning it, keeping it out of
backups, and rotating it invalidates every active session) to the deployment.
Sessions also have a finite lifetime, so even a future weakening of SHA-256
would only expose a bounded window of currently active sessions rather than a
permanent store — further reducing the value of that extra insurance.

## Benefits and trade-offs

**Benefits**:

- No password storage,
- simple deployment,
- reduced attack surface,
- fast session verification.

**Trade-offs**:

- Dependency on email delivery,
- slight UX friction on first access,
- slightly weaker defense-in-depth against a correlated future compromise (a
  leaked hashing key combined with a weakened hash algorithm) than a keyed hash
  would provide.
