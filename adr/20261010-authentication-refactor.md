# Authentication refactor

_Date:_ 2026-10-10

_Status:_ Draft

## Context

The _multi-user_ version of _activities_ authenticates users through single-use, short-lived
authentication links sent by email (see
[Users authentication strategy](20250918-authentication.md)). Upon successful login, a session token
is issued and kept server-side until it expires.

The authentication state was initially spread across three separate SQLite databases: `user.db` for
the email to user id mapping, `auth_link.db` for the authentication links and `session.db` for the
sessions. The auth link and session databases have since been migrated into the users database,
tying their rows to the user they belong to through a foreign key. Their content was not migrated:
links and sessions only hold temporary data (links expire after 15 minutes, sessions after 30 days),
so users simply have to log in again after the upgrade, and only registered users were preserved.

Two upcoming features now drive a schema change to the users table itself:

- **Email verification**: registration flows will need to track whether an email address has been
  verified (e.g. by clicking an authentication link sent to it), so that password-based login can
  later be restricted to verified addresses.
- **Password authentication**: users will be able to optionally set a password, stored as a hash.

Additionally, tying a single email to each user in `t_users` itself is too rigid: while changing
their email, a user should be able to have both their old (verified) and new (not yet verified)
address registered at the same time.

## Decision

The email address moves to a dedicated `t_emails` table, referencing the user row, with a
verification status:

- a user can have several emails, while an email address remains globally unique: the login flow
  resolves exactly one user from an email address,
- new emails start unverified; emails of existing users were migrated as verified, since they had to
  click an authentication link sent to that address upon registration,
- deleting a user deletes their emails, along with their auth links and sessions.

`password_hash` is added to `t_users` as a nullable column: a user may not have a password. It only
ever stores a password hash, never a plaintext password.

### Authentication method configuration

The two authentication methods are configured and parsed independently, each as an all-or-nothing
group:

- **Email-based auth** — the five `ACTIVITIES_MAILER_*` variables (from, username, password, relay,
  domain) are grouped in `EmailAuthConfig`; setting any of them requires setting all of them, and
  partial mailer configurations are rejected at startup.
- **Password-based auth** — `ACTIVITIES_PASSWORD_AUTH_ENABLED` (`true`/`false`, defaults to
  `false`), parsed into `PasswordAuthConfig` (an empty struct for now: it only models "enabled").

At least one method must be configured, otherwise the application fails to start. `MultiUserConfig`
holds each method as an `Option<...>`, from which the user service derives an
`AuthMethods { email, password }` value; the service branches on which methods are enabled and never
sees SMTP credentials or other mailer details. Multi-user mode itself is selected when any of these
authentication variables is set — previously the mode was implicitly selected by the mailer
variables, which were mandatory in multi-user mode. This lifts that requirement, so a multi-user
deployment can run password-only without any SMTP relay (not yet implemented at the service level).

### Login flow

`UserService::login_user` branches on the active methods and on whether the request carries a
password:

- **Email only**: the user is resolved from the email address and an authentication link is
  generated and sent. The caller always observes the same "email sent" outcome, whether or not the
  account exists; an unknown address still pays for a dummy Argon2 hash so the timing matches the
  found-user path.
- **Email and password**: when a password is supplied it is verified and, on match, a session token
  is issued directly without an email round-trip. When no password is supplied (or the field is
  empty), the flow falls back to the email path above.
- **Password only**: a password is required. On match a session is issued; a missing password fails
  without falling back to email.

Password verification is uniform across all failing cases: unknown address, wrong password, and an
account without a stored password (e.g. registered through the mail-only fallback in mail+password
mode) all perform the same Argon2 verification — against a dummy hash for the first and third cases
— and collapse into a single `Invalid` outcome that the HTTP layer renders as a generic
`401 Unauthorized`. The result value, response and timing are therefore indistinguishable, so the
endpoint leaks neither account existence nor password presence.

## Alternatives considered

- **A `verified` column on `t_users` instead of a dedicated table.** Rejected: it would keep the
  one-email-per-user assumption baked into the schema, and verification state naturally belongs to
  the address, not to the user.
- **A unique constraint on the `t_emails.user` column** to enforce one email per user. Rejected: it
  would prevent the email-change flow, where the old and new addresses coexist until the new one is
  verified.
- **Keeping `password_hash` in a dedicated table.** Rejected for now: a user has at most one
  password, so a nullable column on `t_users` is simpler; it can be extracted later if credentials
  ever multiply (e.g. passkeys).
- **Keeping the mailer fields flat on `MultiUserConfig`.** Rejected: the enabled authentication
  methods were only implicit (the config existed iff the mailer was configured), and password-based
  auth had nowhere to live. Grouping each method in its own struct makes "email auth configured"
  explicit and keeps partial configurations invalid.
- **A boolean toggle for email auth too** (e.g. `ACTIVITIES_EMAIL_AUTH_ENABLED`) for symmetry with
  `ACTIVITIES_PASSWORD_AUTH_ENABLED`. Rejected for now: mailer credentials are already required to
  operate email auth, so an explicit enable flag would be redundant; password auth needs one only
  because its configuration is currently empty.
- **Requiring both methods to be configured**, letting operators disable one via its toggle.
  Rejected: it would force every multi-user deployment to supply SMTP credentials even when only
  password auth is used.
- **Migrating the auth link and session content** during the merge. Rejected: it only holds
  ephemeral data, so users get logged out instead, which keeps the migrations simple.

## Benefits and trade-offs

**Benefits**: per-email verification status, several emails per user, and a home for password
hashes, with referential integrity to the user enforced by the database and automatic cleanup of
auth state on user deletion.

**Trade-offs**: users have to log in again after the upgrade; email uniqueness now lives on
`t_emails` and user-by-email lookups require a join. New registrations produce unverified emails
until a verification flow is implemented.

Supporting the authentication methods independently adds its own trade-offs: multi-user mode
selection is based on environment-variable presence, so even explicitly disabling password auth
(e.g. `ACTIVITIES_PASSWORD_AUTH_ENABLED=false` while configuring the mailer) keeps the application
in multi-user mode rather than falling back to single-user. With both methods optional, the service
must handle every combination (email-only, password-only, both): password-only registration cannot
verify the email through a link, so accounts register with unverified email addresses until a
verification flow exists, and email-only setups silently ignore a password supplied at registration.
Handling every combination also means password verification must not reveal which combination a
given account is in: collapsing unknown users, wrong passwords and accounts without a password into
a single `Invalid` outcome (with a dummy Argon2 verification to equalize timing) keeps the endpoint
uniform, at the cost of not being able to tell the user _why_ a login failed.

## Addendum: user account settings (2026-11-08)

Building on the flows and schema above, logged-in users can now manage their own account: change
their email address, and set or change their password, depending on which authentication methods the
instance enables.

### Email change

- A logged-in user requests a change to a new address.
- **Email-based auth enabled**: a single-use authentication link (15 minutes expiry, same machinery
  as login links) is generated and sent **to the new address only**, to confirm it is valid and
  reachable. The link carries the target email as a payload (`t_auth_links.new_email`, `NULL` for
  login links). On validation the user's address is swapped transactionally and marked verified; the
  old address stops working at that point. Until then, the old address keeps working.
- **Email-based auth disabled** (password-only instance): no mailer is configured, so the change
  applies instantly and the new address is marked verified.
- The new address must not already belong to another account (`409 Conflict`).

### Password management

Password management is split in two operations, each with a crisp invariant:

- **Update password** (`POST /api/user/password`): requires the current password; works only if the
  user already has one. A wrong current password collapses into the same uniform `Invalid` outcome
  as login, with the same dummy-hash timing equalization.
- **Register password** (`POST /api/user/password/register`): takes only a new password; works only
  if the user has **no** password yet.

The register path exists for accounts created before password auth was enabled (they have no stored
password), so that migrating an instance email-only → email+pwd → pwd-only does not lock those users
out once password mode activates. Both endpoints require a valid session, so an anonymous attacker
cannot register a password on someone else's account. Registration in pwd-only mode already requires
a password, so accounts created after the migration are unaffected.

**Residual risk**: registering a password only helps users who still have a valid session. A user
whose session expired after the instance dropped email auth cannot be rescued (pwd-only mode has no
mailer, hence no login link). Operators migrating away from email auth should always run an
email+pwd phase long enough for users to log in via an email link and register a password. A future
admin role may provide a stronger remedy.

### Sessions

Neither an email change nor a password change invalidates existing sessions; they remain valid until
their normal expiry. Revocation is strictly manual instead: an authenticated user can **sign out
other devices** (`POST /api/account/sessions/revoke-others`), which deletes every session except the
one used for the request, or **sign out everywhere** (`POST /api/account/sessions/revoke-all`), which
deletes every session and clears the session cookie.

The current session is identified by the hash of the *effective* session token, recorded on the
request by the cookie middleware: `check_session_token` may have just rotated the token, in which
case the request cookie still carries the old (now deleted) one. Revoking others must exclude the
refreshed session, not the cookie's, or it would log the caller out. Conversely, a handler that
revokes the current session (`revoke-all`) clears the cookie and marks the response so the
middleware does not append a rotated cookie over the removal.

**Residual risk and alternative considered**: keeping revocation manual means a user who changes
their password does not automatically cut off a session an attacker may already hold; they have to
remember to sign out other devices. A plausible future default is to automatically revoke *other*
sessions on password change (keeping the current one, hence no surprise logout) while leaving email
changes manual. It was deliberately left out to keep revocation entirely user-triggered, but the
`revoke_other_sessions` primitive already supports it.

### Email verification

An authenticated user whose current address is unverified can request a fresh verification link
(`POST /api/account/email/verify`) when email-based auth is enabled. The endpoint reuses the login
auth-link machinery — same single-use token, 15-minute expiry, background send and rate limit — and
is a no-op (`200 OK`) when the address is already verified; it is rejected (`403 Forbidden`) when
email auth is disabled. Each auth link is bound to the address it was sent to (`t_auth_links.email`),
and consuming a link marks that exact address verified in the same transaction that deletes the
token, using the address read back from the deleted row: opening a link proves control of the
address it was mailed to, and a link can neither be used twice nor verify an address the user
changed away from in the meantime (the update simply matches no row). This also closes the
registration loop, since registration already sends an auth link. Links created before the migration
carry no bound address and are consumed without verifying anything.

### Client surface

A `/settings` page (password section, switching between "set" and "change" based on a
`GET /api/user/password` status endpoint, hidden when password auth is disabled) and a dedicated
`/settings/email` page hosting both the change form, a "verify email" action shown while the address
is unverified, and the auth-link token validation (token in the URL fragment, same pattern as
`/login#token`).

### Alternatives considered (account settings)

- **A single "change password" endpoint with an optional current-password field.** Rejected: fuzzy
  semantics (what does an absent current password mean?) and a bypass risk; the split keeps each
  endpoint's precondition explicit.
- **Asking both the old and the new address to confirm an email change.** Rejected for now: the old
  address demonstrably works (the user is logged in with a live session); requiring two
  confirmations adds friction without a security benefit in this threat model.
- **A dedicated table for email-change links** instead of a payload column on `t_auth_links`.
  Rejected: links are single-use, short-lived and already carry the user reference; a nullable
  column reuses validation, expiry and cleanup unchanged.

## Addendum: password authentication is always enabled (2026-10-13)

### Context

Multi-user mode originally supported three configurations: email-only, password-only, and both.
Password auth was off by default (`ACTIVITIES_PASSWORD_AUTH_ENABLED` omitted) and the two methods
were independent, so an operator could move from email-only to password-only. Accounts created in
email-only mode have no `password_hash`, and an account cannot log in with a password it never set:
switching to an exclusive password mode therefore locked those users out. The email-only →
email+password → password-only migration path (with the "register password" flow) already mitigated
this, but it relied on operators following it and on users keeping a valid session long enough to
register a password.

### Decision

Password-based authentication is now **always enabled** in multi-user mode. Email-based
authentication remains an optional addition on top.

- `MultiUserConfig` no longer carries a password method: only the optional `email` config and
  `allow_registration` remain. `AuthMethods` tracks only whether email auth is enabled;
  `/api/auth_info` keeps reporting `auth_methods.password = true` so existing clients are unaffected.
- **Registration** always requires a password: a missing password answers `PasswordRequired`
  (`400`), and the hash is stored with the user. When email auth is enabled, the authentication link
  is still generated and sent, both to verify the address and to log the user in.
- **Login** accepts a password whenever one is supplied. When none is supplied it falls back to the
  authentication-link flow only if email auth is enabled; otherwise the result is the usual uniform
  `Invalid`. Unknown accounts, wrong passwords and passwordless accounts keep collapsing into the
  same `Invalid` outcome with the same dummy-Argon2 timing equalization.
- `ACTIVITIES_PASSWORD_AUTH_ENABLED` is kept only as a multi-user **mode selector** (notably to run
  password-only without a mailer). Its value no longer disables anything: an explicit `false` is
  accepted for backward compatibility and logs a warning at startup instead of failing, so existing
  email-only deployments upgrade without breaking. An invalid value still fails fast.
- The now-unreachable `RegisterPasswordResult::PasswordAuthDisabled` and
  `UpdatePasswordResult::PasswordAuthDisabled` variants (and their `403` HTTP mapping) are removed.
- At startup, the application counts users without a `password_hash` and logs a warning with the
  number. The message adapts to whether email auth is enabled: with email auth on it tells the
  operator those users would be locked out if it were disabled and to have them set a password;
  with email auth off it reports that those accounts currently cannot log in. A future admin role
  will provide a stronger remedy.

No schema change is needed: `password_hash` remains nullable, so existing instances keep their
passwordless rows and no data migration is required.

### Alternatives considered

- **Remove `ACTIVITIES_PASSWORD_AUTH_ENABLED` entirely and add a new multi-user selector.** Rejected
  for now: it would break the deployment configuration of every existing password-only instance for
  no functional gain, since the variable already doubles as the selector. Keeping it and ignoring
  its value preserves backward compatibility.
- **Reject `ACTIVITIES_PASSWORD_AUTH_ENABLED=false` at startup.** Rejected: email-only deployments set
  it explicitly and would stop starting on upgrade even though the requested configuration is
  expressible again (email + the now-mandatory password). A warning is preferred.
- **Automatically assign or reset passwords for existing passwordless accounts.** Rejected: it would
  either invent credentials users cannot know, or email one in a flow that password-only instances
  cannot run. Warning operators and relying on the existing register-password flow is safer.
- **Key the warning off a new metric/endpoint instead of startup logs.** Rejected for now: a log line
  is enough to reach self-hosters at upgrade time; exposing the count can come with the admin role.

### Benefits and trade-offs

**Benefits**: new instances can never be configured into a state where a user has no credential
path once email auth is dropped; password login works without a mail server, so password-only
deployments are the simplest to operate; existing passwordless accounts keep working through the
email flow and can be rescued via the register-password flow.

**Trade-offs**: email-only instances necessarily pick up password auth on upgrade, so the password
field appears for everyone and users must now choose a password at registration; the
`ACTIVITIES_PASSWORD_AUTH_ENABLED` name no longer matches its behavior; and the lock-out risk is only
reduced, not removed, for accounts that already exist without a password (the startup warning is the
main safeguard until an admin role exists).
