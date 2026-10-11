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
holds each method as an `Option<...>`, from which the user service derives an `AuthMethods { email,
password }` value; the service branches on which methods are enabled and never sees SMTP
credentials or other mailer details. Multi-user mode itself is selected when any of these
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
mode) all perform the same Argon2 verification — against a dummy hash for the first and third
cases — and collapse into a single `Invalid` outcome that the HTTP layer renders as a generic
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
uniform, at the cost of not being able to tell the user *why* a login failed.
