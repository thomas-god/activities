# Activities: self-hosted sport training journal

Activities is a self-hosted platform to archive sport activities, manage your training journal and
track your progress:

- Import sport activities from `.tcx` and `.fit` files,
- Keep a training journal with notes on workouts and training blocks,
- Explore and track your progress with custom training metrics,
- It can be deployed from a single Docker image,

## How to deploy

Get the latest image available at `ghcr.io/thomas-god/activities:latest`. All images are available
for both `amd64` and `arm64`.

The application authentication flow can be configured in 3 different ways:

- _multi user_: allows multiple users to use your instance. Password-based login is always enabled,
  and email-based password-less login (authentication links sent by email) can be enabled on top. By
  default anyone with a valid email address can register; set `ACTIVITIES_ALLOW_REGISTRATION=false`
  to restrict the instance to already-registered users only (an [admin](#admin-role) can still
  create accounts). Email-based login requires a valid **SMTP server** for sending emails to users
  containing their authentication link. HTTPS is strongly recommended (session cookies are only
  marked `Secure` when served over HTTPS), but the app also works over plain HTTP when the traffic
  is already protected by another layer (e.g. a WireGuard or Tailscale tunnel).
- _single user, main password_: allows a single user to use the instance using a password set
  through an environment variable in your deployment. Does not require having an SMTP server nor
  having HTTPS (HTTP only might be fine for local deployment but HTTPS is still recommended as
  password and cookie will travel in clear otherwise).
- _single user, no password_: allows a single user to use the instance without any form of
  authentication. Be careful where you plan to host it, as your activities will be accessible to
  anyone that can reach your instance. Does not require having an SMTP server nor having HTTPS.

Beside the authentication process, all versions offer the same set of features.

### Quick start: try it locally

You can quickly test the application locally with a single Docker command:

```bash
docker run --rm -p 8080:80 \
  -e ACTIVITIES_DATA_PATH=/app/data \
  ghcr.io/thomas-god/activities:latest
```

Then open http://localhost:8080 in your browser. Your data will be stored in an anonymous Docker
volume and will be lost when you stop the container. To persist your data, add a volume mount:

```bash
docker run --rm -p 8080:80 \
  -e ACTIVITIES_DATA_PATH=/app/data \
  -v activities_data:/app/data \
  ghcr.io/thomas-god/activities:latest
```

### Using _docker-compose_ or _docker-stack_

The most basic `docker-compose.yaml` file looks like that:

```yaml
services:
  activities:
    image: ghcr.io/thomas-god/activities:latest
    environment:
      ACTIVITIES_DATA_PATH: /app/data
    volumes:
      - activities_data:/app/data

volumes:
  activities_data:
```

The application will store all its data at the location pointed by `ACTIVITIES_DATA_PATH`, so use a
docker volume to persist your data between restarts and to facilitate the backup process.

### HTTPS is recommended, plain HTTP is possible on a trusted network

The bundled image only serves plain HTTP and does not terminate TLS by itself. Session cookies are
marked `Secure` only when the request reaches the backend over HTTPS — as reported through the
`X-Forwarded-Proto` header by a TLS-terminating reverse proxy (e.g.
[Caddy](https://caddyserver.com/) or [Traefik](https://traefik.io/)). Over plain HTTP the cookie is
still set, so login works, but it then travels in clear. Only run the _multi user_ (or _single user,
main password_) modes over HTTP when the traffic is already protected by another layer, for example
a [WireGuard](https://www.wireguard.com/) or [Tailscale](https://tailscale.com/) tunnel.

A minimal `docker-compose.yaml` with Traefik handling TLS looks like:

```yaml
services:
  activities:
    image: ghcr.io/thomas-god/activities:latest
    environment:
      ACTIVITIES_DATA_PATH: /app/data
      ACTIVITIES_SINGLE_USER_PASSWORD: my-secret
    volumes:
      - activities_data:/app/data
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.activities.rule=Host(`your.domain`)"
      - "traefik.http.routers.activities.entrypoints=websecure"
      - "traefik.http.routers.activities.tls.certresolver=letsencrypt"
      - "traefik.http.services.activities.loadbalancer.server.port=80"

  traefik:
    image: traefik:v3.2
    command:
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge.entrypoint=web"
      - "--certificatesresolvers.letsencrypt.acme.email=you@your.domain"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - traefik_letsencrypt:/letsencrypt

volumes:
  activities_data:
  traefik_letsencrypt:
```

### Configuration

The application accept the following environment variables to be set. Each environment variable
supports a `*_FILE` variant to load the variable content from a file
[instead](https://docs.docker.com/engine/swarm/secrets/#build-support-for-docker-secrets-into-your-images).

#### All versions

| Variable name        | Required | Purpose                            | Example   |
| -------------------- | -------- | ---------------------------------- | --------- |
| ACTIVITIES_DATA_PATH | yes      | Location to store application data | /app/data |

#### Single user versions

| Variable name                   | Required | Purpose                                                                                                                                            | Example   |
| ------------------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| ACTIVITIES_SINGLE_USER_PASSWORD | no       | Add a required password to access the instance. If not set, there will be no authentication in place (falling back to _single user, no password_). | my-secret |

#### Multi-user version

Multi-user mode is enabled as soon as any multi-user environment variable below is set:

- **password-based authentication is always enabled**: users register with a password and log in
  with it;
- **email-based authentication is optional**: configuring all the `ACTIVITIES_MAILER_*` variables
  adds password-less login through authentication links (and email verification);
- **an admin role is available**: the first admin is established from the environment or with the
  CLI, and is the account used to manage users when registration is closed (see
  [Admin role](#admin-role)).

A password-only instance runs without any SMTP configuration: set the
`ACTIVITIES_PASSWORD_AUTH_ENABLED` variable (its value no longer matters) to select multi-user mode,
or simply configure the mailer for email-based login as well.

| Variable name                       | Required             | Purpose                                                                                                                                                                                                                                                                                                                       | Example                 |
| ----------------------------------- | -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| ACTIVITIES_MAILER_FROM              | for email-based auth |                                                                                                                                                                                                                                                                                                                               | no-reply@your.domain    |
| ACTIVITIES_MAILER_USERNAME          | for email-based auth | SMTP username                                                                                                                                                                                                                                                                                                                 |                         |
| ACTIVITIES_MAILER_PASSWORD          | for email-based auth | SMTP password                                                                                                                                                                                                                                                                                                                 |                         |
| ACTIVITIES_MAILER_RELAY             | for email-based auth | SMTP server                                                                                                                                                                                                                                                                                                                   |                         |
| ACTIVITIES_MAILER_DOMAIN            | for email-based auth | The domain on which your instance is hosted. Used to craft the auth link url. Can be a different domain than the one used by your SMTP server.                                                                                                                                                                                | https://app.your.domain |
| ACTIVITIES_PASSWORD_AUTH_ENABLED    | no                   | Deprecated selector kept for backward compatibility. Password-based authentication can no longer be disabled: the value is ignored and setting it to `false` only logs a warning. It still selects multi-user mode, so it can be used on its own to run a password-only instance without the `ACTIVITIES_MAILER_*` variables. | true                    |
| ACTIVITIES_ALLOW_REGISTRATION       | no                   | Wether new users can register a new account. Set to `false` to only allow already-registered users to log in.                                                                                                                                                                                                                 | false                   |
| ACTIVITIES_BOOTSTRAP_ADMIN_EMAIL    | no                   | Email of the account to create or promote as the first admin at startup. Must be set together with the password variable; the bootstrap only runs while the instance has no admin.                                                                                                                                            | admin@your.domain       |
| ACTIVITIES_BOOTSTRAP_ADMIN_PASSWORD | no                   | Password for the bootstrapped admin account. Set together with the email variable. A `*_FILE` variant is recommended to keep the secret out of the deployment configuration.                                                                                                                                                  |                         |

_Notes:_

- _if any environment variables for the multi-user version is set but others are missing the
  application will fail to start, even if a valid single user configuration could be loaded._
- _password-based authentication can no longer be disabled: `ACTIVITIES_PASSWORD_AUTH_ENABLED=false`
  is accepted for backward compatibility but ignored (a warning is logged), and password login comes
  on. The variable is still what selects multi-user mode, so a password-only instance can run
  without any mailer. If you intend to run with no authentication at all (*single user, no
  password*), leave **all** multi-user variables unset._
- _at startup, a warning reports how many existing users have no password registered (if your
  instance pre-dates the authentication update). Those accounts can only log in through the
  email-based flow; make sure they set a password from their account settings while email login is
  still enabled, otherwise disabling email-based authentication will lock them out. An
  [admin](#admin-role) can also set a password for those accounts._

#### Admin role

An **admin** is a regular account flagged as administrator. The first admin is established while the
instance has no admin yet, either:

- **from the environment** — set both `ACTIVITIES_BOOTSTRAP_ADMIN_EMAIL` and
  `ACTIVITIES_BOOTSTRAP_ADMIN_PASSWORD` (or `ACTIVITIES_BOOTSTRAP_ADMIN_PASSWORD_FILE`) before
  starting the container. The account is created if it does not exist, and promoted otherwise (with
  its password updated). Once an admin exists the variables become inert, so remove them from your
  configuration afterwards.
- **with the CLI** — `app admin ...` runs directly against the database, without a session and
  without the rest of the server configuration. It is also the break-glass path when the server
  cannot start or the current admin is locked out:

  ```bash
  # Create or promote an admin. Prefer --password-file to keep the secret out of the process list.
  docker exec -it <container> /app_bin admin bootstrap \
    --email admin@your.domain --password-file /run/secrets/admin

  # Clear the admin flag (refused if it would leave the instance with no admin).
  docker exec -it <container> /app_bin admin revoke --email admin@your.domain
  ```

An admin can manage accounts from the **`/admin`** page (linked in the navigation once logged in as
an admin): create users even when registration is closed, set or reset any user's password, grant or
revoke the admin role, and revoke a user's sessions. The first admin still has to exist before that
page can be reached, hence the bootstrap variables and CLI above; the CLI remains the break-glass
path and also refuses to revoke the last admin.

#### Observability

`OTEL_EXPORTER_OTLP_ENDPOINT` and `OTEL_SERVICE_NAME` go through the application's own config loader
like the variables above, so the `*_FILE` variant described above works for them too (e.g.
`OTEL_EXPORTER_OTLP_ENDPOINT_FILE`). They keep the standard OpenTelemetry names (rather than an
`ACTIVITIES_` prefix) for interoperability with collectors/dashboards that expect them. `RUST_LOG`
is read directly by the `tracing` ecosystem instead, since it's a generic `tracing`/`log` convention
rather than something specific to this app (the `*_FILE` variant does not apply to it).

| Variable name               | Required | Purpose                                                                                                                                                                              | Example                    |
| --------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------- |
| OTEL_EXPORTER_OTLP_ENDPOINT | no       | OTLP/HTTP (protobuf) collector base endpoint — `/v1/traces` is appended automatically. When unset, tracing spans are only logged to stdout and no OpenTelemetry exporter is started. | http://otel-collector:4318 |
| OTEL_SERVICE_NAME           | no       | `service.name` resource attribute reported on exported spans. Defaults to `activities-app`.                                                                                          | activities-app             |
| RUST_LOG                    | no       | Log/span level filter ([`EnvFilter` syntax](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)). Defaults to `info`.                         | app=debug,tower_http=info  |

## Upload your training history

Most device manufacturers and training platforms allow you to download your activities history:

- _Coros_: Training Hub > Activity List > Export Data
- _Garmin Connect_: Account Settings > Account Information > Export all of your Garmin data
- _Strava_: Settings > My Account > Download or Delete Your Account

They will usually send you a link by email to download an archive containing all your data.
Depending on the archive format you should find an activities folder containing `.fit` and or `.tcx`
files, often compressed in `.gz`.

Then simply upload those files (including compressed ones) using the upload form. It can take a few
minutes depending of your history size but then you are good to go!

![history-upload](resources/history-upload.png)

## Intended scope and limitations

- The application focuses on providing athletes with tools to track and analyze their training:
  organizing activities into training periods, keeping training notes, defining custom metrics, and
  visualizing performance trends. While social features like friends, timeline, or sharing are not
  planned, coaching-related features might be considered in the future.
- Because the application is designed to be self-hosted, and since sport device ecosystems are
  mostly closed, there is sadly no feature to automatically import your activities from your
  devices.

## Design decisions

Key design decisions are documented [here](adr/README.md).

## Credits

Favicon from [OpenMoji](https://openmoji.org/) – the open-source emoji and icon project. License:
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/)
