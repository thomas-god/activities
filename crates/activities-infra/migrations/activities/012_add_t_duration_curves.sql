CREATE TABLE IF NOT EXISTS t_duration_curves (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    activity_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    date TEXT NOT NULL,
    type TEXT, -- power | pace | null if activity does not have any valid curve
    secs_5 REAL,
    secs_10 REAL,
    secs_30 REAL,
    mins_1 REAL,
    mins_2 REAL,
    mins_5 REAL,
    mins_10 REAL,
    mins_20 REAL,
    mins_30 REAL,
    hours_1 REAL,
    hours_2 REAL,
    hours_5 REAL,
    UNIQUE(activity_id, user_id, type)
);

-- Rows with a NULL type mark activities that were processed but have no valid duration curve.
-- NULL values are never considered conflicting by a UNIQUE constraint, so this partial index
-- ensures only one marker row exists per (activity, user).
CREATE UNIQUE INDEX IF NOT EXISTS u_duration_curves_null_type
ON t_duration_curves (activity_id, user_id)
WHERE type IS NULL;

-- Add matching outbox table for notifying the training service
CREATE TABLE IF NOT EXISTS t_outbox_duration_curve (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    activity_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    event TEXT NOT NULL, -- added/deleted
    occurred_at TEXT NOT NULL,
    processed_at TEXT
);
