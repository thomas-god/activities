# Users authentication strategy

_Date:_ 2025-09-18

## Context

The _multi-user_ version of the applications needs to support user
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
- Session tokens are cryptographically random (192-bit),
- Sessions expire after 30 days of inactivity,
- Email is assumed to be a reasonably secure channel for our threat model.

## Benefits and trade-offs

**Benefits**: No password storage, simple deployment, reduced attack surface
**Trade-offs**: Dependency on email delivery, slight UX friction on first access
