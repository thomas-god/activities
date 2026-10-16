# Database backups

_Date:_ 2026-10-16

_Status:_ Draft

## Context

All of _activities_' data lives under a single directory pointed to by `ACTIVITIES_DATA_PATH`, split
between two kinds of files:

- the raw activity files (`.fit`, `.tcx`, ...) under `activities/`,
- the SQLite databases under `db/`: `activities.db`, `training_metrics.db`, `preferences.db`,
  `search.db` and `user.db`.

The raw activity files are regular binary files, so backing them up is a simple file copy. The
SQLite databases are not: they run in [WAL mode](https://www.sqlite.org/wal.html), where committed
data can still live in a separate `-wal` file, and a naive binary copy taken while the application
is running can capture the main database file and its WAL at different points in time. The result is
a backup that is either corrupted or silently rolled back, which is only discovered at restore time
— the worst possible moment.

We want a backup mechanism dedicated to the database files that produces consistent snapshots
without stopping the application, on a configurable schedule.

## Key decisions and implementation details

### Use SQLite's `VACUUM INTO` to produce snapshots

For each database, the backup opens a dedicated connection and runs `VACUUM INTO '<destination>'`.
`VACUUM INTO` (available since SQLite 3.27, and the version bundled with the app is far newer) reads
a transactionally consistent snapshot of the database and writes it as a new, defragmented,
single-file database. It does not modify the source, does not block writers, and does not require
checking the WAL beforehand. The resulting file is a valid, self-contained SQLite database that is
safe to copy with any regular file tool.

The backup is written to a temporary file and atomically renamed once complete, so an interrupted
run never leaves a file that looks like a valid backup.

`PRAGMA integrity_check` is run against each produced file before it is added to the archive. This
catches a bad snapshot at backup time rather than at restore time, at the cost of a slightly slower
run.

#### Trade-offs and alternatives considered

- **Copying the database, `-wal` and `-shm` files.** This is the situation we are trying to fix: not
  transaction-aware and can produce corrupt backups.
- **The SQLite Online Backup API** (`sqlite3_backup_*`) is the "official" tool for this and supports
  incremental page-level copies. However `sqlx` does not expose it, so using it would mean pulling
  in a second SQLite binding (e.g. `rusqlite`) alongside the one used by `sqlx`, with all the
  duplicated native library and version-management complexity that implies. `VACUUM INTO` provides
  the transactional guarantee we need with no additional dependency.
- **Shelling out to the `sqlite3` command line tool** (`sqlite3 ... .backup`) would require shipping
  the CLI in the runtime image and managing a child process. Rejected in favor of an in-process
  query.
- `VACUUM INTO` requires temporary free space roughly equal to the size of the database being backed
  up, and rewrites the whole database on each run (no incremental backup). Given the data volumes
  involved this is an acceptable trade-off, and it is the same space cost the Online Backup API
  would pay for a full copy.

### Backups live in a dedicated directory, but not necessarily a dedicated volume

Snapshots are written to a configurable directory, defaulting to `<ACTIVITIES_DATA_PATH>/backups`,
overridable with `ACTIVITIES_DB_BACKUP_PATH`.

Writing the snapshots to a directory distinct from `db/` keeps the live databases and the recovery
points separate, makes retention straightforward, and avoids any risk of the application opening a
snapshot as if it were a live database. It does **not**, by itself, protect against the loss of the
underlying disk or volume.

It would be tempting to require the backup directory to be a separate mounted volume, but the
application cannot verify or enforce the mount topology from inside the container, and a separate
volume that happens to live on the same physical disk gives a false sense of safety. More
importantly, requiring an extra volume would break the out-of-the-box deployment for users who never
configure one. The application therefore produces consistent snapshots at a configurable location,
and the README documents that true off-host protection still requires pointing that location at a
separate mount and/or replicating it externally (e.g. with `restic`, `rsync`, or a volume-level
tool).

#### Trade-offs and alternatives considered

- **Forcing a separate volume.** Considered, but rejected because it trades a working default for a
  guarantee the application cannot actually make.
- **Storing snapshots next to the live databases in `db/`.** Considered, but rejected to keep
  recovery points and live files clearly separated.

### One archive per run

A run snapshots every source-of-truth database and packages them, together with a `manifest.json`
(timestamp, list of databases, sizes, application/SQLite versions), into a single
`backup-<timestamp>.zip` file. The zip is built under a temporary name and atomically renamed when
complete, so a partial archive is never mistaken for a good one.

Archiving loses the ability to grab a single database file directly, but makes each recovery point a
single, easy-to-copy artifact, which is what matters for the off-host replication step described
above.

#### Trade-offs and alternatives considered

- **One file per database, no archive.** Considered. Simpler to inspect, but a recovery point
  becomes a directory of many files that must be kept together and copied consistently, which pushes
  complexity onto the operator.

### Retention

The N most recent archives are kept (`ACTIVITIES_DB_BACKUP_RETENTION`, default 7; `0` keeps
everything). Retention is applied after a successful run. Without it, backups would grow unbounded
and eventually fill the disk, which is itself a cause of database corruption.

### Scheduling: an in-process cron, built on a one-shot command

The backup logic is implemented as a single "run once" operation, exposed both as the
`app backup run` command and called by a small in-process scheduler when `ACTIVITIES_DB_BACKUP_CRON`
is set (a standard cron expression, e.g. `0 3 * * *`). When the variable is unset, no scheduler is
started at all.

This keeps the backup and scheduling concerns separate: the command is the engine, the scheduler is
a thin loop that waits for the next occurrence and calls it. It also serves operators who already
have a host scheduler (host crontab, a systemd timer, Docker `ofelia`, a Swarm or Kubernetes
`CronJob`): they can invoke `app backup run` from their existing tooling instead of relying on the
application's own timer. Runs are allowed to overlap: each is self-contained (a unique staging
directory and archive name) and SQLite permits concurrent read snapshots of a live database, so a
scheduled run and a manually triggered one — typically a separate process — do not need to be
serialized. The only cost is a temporarily higher peak disk usage while two snapshots coexist.

#### Trade-offs and alternatives considered

- **Scheduler only.** Would force operators with existing scheduling infrastructure to duplicate it
  inside the container, and would make manual backups and integration tests go through the
  scheduler.
- **Command only (delegate all scheduling to the host).** Would require every deployment to
  configure an external scheduler, raising the barrier to entry for the default Docker deployment.
- **Serializing runs with a cross-process lock.** Considered, to cap peak disk usage and fail fast
  when a run overlaps another. Rejected: it is not needed for correctness (see the concurrency
  note above), the worst retention race only results in a missing-file warning or keeping slightly
  fewer archives, and it would introduce a user-visible failure mode where a manual run fails
  because a scheduled one happens to be in progress.
  Providing both keeps the simple case simple.
- **`tokio-cron-scheduler`.** A heavier dependency than needed; a cron parser plus a `sleep_until`
  loop reuses the scheduling style already used by the application's other background tasks.

### Scope: source-of-truth databases only

`activities.db`, `training_metrics.db`, `preferences.db` and `user.db` are backed up. `search.db` is
explicitly excluded by default because it is a derived index that can be rebuilt from the activities
and training notes, and can be the largest database. It can be included with an opt-in setting for
operators who prefer to avoid the rebuild.

## Consistency across databases

The databases are backed up sequentially, so each snapshot is taken at a slightly different instant.
There are no cross-database foreign keys — links are logical (`user_id`, `activity_id`, `period_id`)
— so this raises the question of what inconsistency this skew can actually produce.

- **Each file is always valid.** `VACUUM INTO` guarantees an internally consistent database; the
  skew is only semantic, never a corrupt file.
- **Users.** There is no user-deletion feature, so the realistic window is a user created between
  snapshots, in which case the other databases simply have no rows for them yet. No orphaned data
  can be produced.
- **Activities vs. training duration curves.** The same `activity_id` appears in `activities.db` and
  in `training_metrics.db`, where the latter is populated asynchronously through an outbox. A skew
  can leave one side updated and not the other.
- **The outbox makes this self-healing.** Both databases carry outbox tables with a `processed_at`
  column, and the apply paths are idempotent (upserts, idempotent deletes, idempotent "mark as
  processed"). On restart after a restore, pending events are replayed and the two sides converge.
- **Sessions and tokens.** They live in `user.db` and are consistent with the users in the same
  file. Restoring an older copy could resurrect a revoked session or a consumed login link, but
  those expire and the restore is a deliberate operator action.

Because the only real skew is self-healing through the outbox, no attempt is made to take a globally
atomic snapshot (which would require pausing all writes). Taking all snapshots within the same run,
back to back, is sufficient.

## Benefits and trade-offs

**Benefits**: transactionally consistent backups without stopping the application, no new native
dependency, an easy-to-copy recovery artifact, a bounded on-disk footprint, and a mechanism that
works both on a schedule and on demand / from an external scheduler.

**Trade-offs**: each run copies the full databases (no incremental backup) and temporarily needs
free space roughly equal to their total size; the snapshots of different databases are not
atomically consistent with each other, which is acceptable because the outbox replays pending work
on restart; off-host durability still depends on the operator replicating the backup directory,
because the application cannot enforce that the destination is a separate volume.
