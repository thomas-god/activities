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
- **Migrating the auth link and session content** during the merge. Rejected: it only holds
  ephemeral data, so users get logged out instead, which keeps the migrations simple.

## Benefits and trade-offs

**Benefits**: per-email verification status, several emails per user, and a home for password
hashes, with referential integrity to the user enforced by the database and automatic cleanup of
auth state on user deletion.

**Trade-offs**: users have to log in again after the upgrade; email uniqueness now lives on
`t_emails` and user-by-email lookups require a join. New registrations produce unverified emails
until a verification flow is implemented.
