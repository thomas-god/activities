CREATE TABLE IF NOT EXISTS t_activities_v2 (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT UNIQUE,
    user_id TEXT,
    name TEXT NULLABLE,
    start_time TEXT,
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
    id, user_id, name, start_time, sport, natural_key, rpe, workout_type, nutrition, feedback
)
SELECT id, user_id, name, start_time, sport, natural_key, rpe, workout_type, nutrition, feedback
FROM t_activities;