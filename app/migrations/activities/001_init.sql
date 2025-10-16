CREATE TABLE IF NOT EXISTS t_activities (
    id TEXT UNIQUE,
    user_id TEXT,
    name TEXT NULLABLE,
    start_time TEXT,
    sport TEXT,
    statistics BLOB,
    natural_key TEXT
);

CREATE INDEX IF NOT EXISTS t_activities_natural_key_idx
ON t_activities(natural_key);