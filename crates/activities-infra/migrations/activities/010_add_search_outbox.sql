CREATE TABLE IF NOT EXISTS t_outbox_activity_search (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    activity_id TEXT NOT NULL,
    event TEXT NOT NULL,
    content TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    processed_at TEXT
);
