# Architecture decision records

This folder contains architecture decision records (ADR) that detail some key design decisions taken
during the development of _activities_. They usually describe the context of the decision, the
alternative options considered and the resulting trade-offs.

- [User authentication](20250918-authentication.md)
- [Activity storage](20250924-activities-storage.md)
- [Training metrics values computation and storage strategy](20251110-training-metrics-computation.md)
- [Maintaining feature availability and user experience across screen widths and platforms](20251121-screen-width-and-user-experienc.md)
- [Computation of activity metrics derived from timeseries](20260216-activity-timeseries-metrics-computation.md)
- [Client Component Organization Strategy](20260808-client-component-organization.md)
- [Search feature](20260906-search.md)
- [Feedback, weight and nutrition training contexts](20260914-training-contexts.md)
- [Duration curves](20261005-power-curves.md)
- [User authentication refactor](20261010-authentication-refactor.md)
- [Database backups](20261016-database-backups.md)

## High-level tech stack and architecture

_Activities_ uses a classic client/server architecture. The client is written using Svelte, and
SvelteKit with its static adapter. The server uses tokio as its async runtime and axum as its HTTP
crate. The data layer uses SQLite directly bundled to the application in order to avoid having to
deploy and maintain an external dependency. _Activities_ is provided as a single Docker image, with
ngninx serving the client static files alongside the application listening for requests.

The server uses hexagonal architecture to decouple core domain logic (activity, training) from
infrastructure concerns (HTTP API, database schemas) and other supporting domains (authentication,
search, etc.). Domain modules expose traits that are implemented by services (to expose behavior)
and repositories (that own their respective data layers).

Client-driven interactions follows a request/response model: from request parsing, to executing
service logic and waiting for repository's data, the user gets a response within the same
interaction. This is independent from the server execution model (async/await, tokio tasks).

Some work is done in the background, decoupled from any client requests: history processing jobs,
authentication token clean-up jobs, and in-process message passing and processing from domain to
domain (implemented using the outbox pattern and tokio's Notify).
