DROP TABLE IF EXISTS t_outbox_training_search;

CREATE TABLE IF NOT EXISTS t_outbox_training_search (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    note_id TEXT NOT NULL,
    user TEXT NOT NULL,
    event TEXT NOT NULL,
    content TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    processed_at TEXT
);
