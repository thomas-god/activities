CREATE TABLE
    IF NOT EXISTS t_training_notes (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        content TEXT NOT NULL,
        created_at TEXT NOT NULL
    );

CREATE INDEX IF NOT EXISTS idx_training_notes_user_id ON t_training_notes (user_id);