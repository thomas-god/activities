DROP TABLE t_activities_v2;
DROP TABLE t_activities_metrics_values;

CREATE TABLE IF NOT EXISTS t_activities_v2 (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT UNIQUE,
    user_id TEXT,
    name TEXT NULLABLE,
    start_time TEXT,
    duration FLOAT,
    sport TEXT,
    natural_key TEXT,
    rpe INTEGER NULLABLE,
    workout_type TEXT,
    nutrition BLOB,
    feedback TEXT
);

CREATE INDEX IF NOT EXISTS t_activities_v2_natural_key_idx
ON t_activities_v2(natural_key);

INSERT INTO t_activities_v2 (
    id, user_id, name, start_time, duration, sport, natural_key, rpe, workout_type, nutrition, feedback
)
SELECT
    id, user_id, name, start_time,
    coalesce(json_extract(statistics, '$.Duration'), 0.0) as duration,
    sport, natural_key, rpe, workout_type, nutrition, feedback
FROM t_activities;

CREATE TABLE IF NOT EXISTS t_activities_metrics_values (
    activity_rowid INTEGER,
    metric_rowid INTEGER,
    value REAL,
    FOREIGN KEY (activity_rowid) REFERENCES t_activities_v2(rowid) ON DELETE CASCADE,
    FOREIGN KEY (metric_rowid) REFERENCES t_activities_metrics(rowid) ON DELETE CASCADE,
    UNIQUE (activity_rowid, metric_rowid)
);
