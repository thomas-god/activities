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

### Email as the user-facing identifier

Email has been chosen as the user-facing identifier. This allows migrating existing instances (that
necessarily has an email per user) and to more easily enable email-based authentication to new
instances. New instances start with only password-based authentication won't have their users' email
verified since no mailer would have been setup. When enabling email-based authentication users are
still able to login using their previously registered password.

### Support for not set (NULL) passwords

Passwords may be null to support migrating existing instances, while new registrations require one.
Users without a password can register one in their _settings_ page provided they have a valid
session. Login remains possible without a password, provided email-based authentication is still
enable. It is possible to lock users out of their account if email-based authentication is disabled
while they are still users without a password. A warning is logged at startup is there is still such
users, plus the admin page show the same information (see [admin role](#new-admin-role)).

### Verifying existing email

When enabling email-based authentication to a password-only instance there might be users with
invalid (as in unreachable) email. To provide a way to detect those early an email is considered
`unverified` until it has gone through an email authentication flow (receiving an authentication
link). Users can see from their settings page is their email is considered `unverified` and can
manually trigger a verification email.

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
