CREATE TABLE t_api_keys (
    rowid INTEGER PRIMARY KEY,
    key_id TEXT UNIQUE NOT NULL,
    user INTEGER NOT NULL REFERENCES t_users (rowid) ON DELETE CASCADE,
    name TEXT NOT NULL,
    permission TEXT NOT NULL,
    key_hash TEXT UNIQUE NOT NULL,
    display TEXT NOT NULL,
    expire_at TIMESTAMP,
    disabled_at TIMESTAMP
);

CREATE INDEX idx_api_keys_user ON t_api_keys (user);

CREATE TABLE t_api_key_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_key INTEGER NOT NULL REFERENCES t_api_keys (rowid) ON DELETE CASCADE,
    user INTEGER NOT NULL REFERENCES t_users (rowid) ON DELETE CASCADE,
    activity TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL
);

CREATE INDEX idx_api_key_activity_key ON t_api_key_activity (api_key, timestamp);

CREATE INDEX idx_api_key_activity_user ON t_api_key_activity (user);
