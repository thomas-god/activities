# Authentication refactor: password-based authentication and new admin role

_Date:_ 2026-10-10

_Supersedes:_ [Authentication](20250918-authentication.md)

## Context

Currently the _multi-user_ version of _activities_ authenticates users via their email by sending
single-use, short-lived authentication link. While this works well, it requires having a valid SMTP
server to perform the authentication flow. In order to lower the barrier to entry for hosting an
instance of _activities_ we would like to add password-based authentication to the app. One of the
hard requirement when designing this feature is to keep existing _multi-user_ instances (that use
mail-only authentication) running without requiring any update to their deployment configuration.

## Key decisions and implementation details

### Password-base auth by default, email-based auth optional

The default authentication method is password-based and is always available since it requires no
external dependencies. Email-based authentication can be optionally enabled via environment
configuration. When email-based authentication is enabled users can chose to login either using
their password or by receiving a login link to their email (the current authentication flow), thus
not breaking existing instances.

#### Trade-offs and alternatives considered

- Having the ability to enable both modes independently was considered, but it introduced too many
  cases of possibly locking out users when changing authentication mode, and increased code
  complexity for no meaningful added value.

### Email address as the user-facing identifier

Email has been chosen as the user-facing identifier. This allows migrating existing instances (that
necessarily has an email per user) and to more easily enable email-based authentication to new
instances. New instances start with only password-based authentication won't have their users' email
verified since no mailer would have been setup. When enabling email-based authentication users are
still able to login using their previously registered password.

#### Trade-offs and alternatives considered

- Using an email address to register without receiving any email might be awkward from the user
  point of view. This quirk is explicitly explained in the client when email-based authentication is
  disabled.
- A more classic username/password was considered but it would have complicated the migration of
  existing instances and the switch of the mail-based authentication mode for no real benefit.

### Support for not set (NULL) passwords

Passwords may be null to support migrating existing instances, while new registrations require one.
Users without a password can register one in their _settings_ page provided they have a valid
session. Login remains possible without a password, provided email-based authentication is still
enable. It is possible to lock users out of their account if email-based authentication is disabled
while they are still users without a password. A warning is logged at startup is there is still such
users, plus the admin page show the same information (see [admin role](#new-admin-role)).

### Verifying existing email addresses

When enabling email-based authentication to a password-only instance there might be users with
invalid (as in unreachable) email. To provide a way to detect those early an email is considered
`unverified` until it has gone through an email authentication flow (receiving an authentication
link). Users can see from their settings page is their email is considered `unverified` and can
manually trigger a verification email.

#### Trade-offs and alternatives considered

- Considering every email addresses as valid and verified per default was considered as it reduced
  user action, but it introduced some risk as users would realized their email address is invalid
  only when they needed it to receive an authentication link. Surfacing invalid email address early
  diminish the risk of users being locked out of their account.

### Management of existing sessions

Authentication related updates (password, email) do not close existing sessions. Users can
separately close them from their settings page.

### New admin role

To mitigate the risk of locked out users, an admin role has been created that can perform
authentication related operations: checking if users have not set password, changing a user
password, managing admin user, and revoking user's sessions.

A first admin can be bootstrap via configuration from an existing user (or created), and a basic
admin CLI has been added for break-glass access (requires access to the deployment environment, see
the [README](../README.md#admin-role))

#### Admin action audit log

Every admin action — through the web API, the break-glass CLI or the startup bootstrap — is
recorded in `t_admin_audit_log`. Rows capture the origin, the acting user id (when there is one),
the action, the target and the outcome, including refused operations (unknown user, last admin, …).

The actor and the target are denormalized snapshots without a foreign key to `t_users`: the CLI and
the bootstrap have no user account, and the trail must survive the deletion or renaming of the
users it mentions. Each row is written in the same transaction as the mutation it describes, so an
action and its audit entry always stay consistent. Secrets (passwords, hashes, tokens) are never
stored.
