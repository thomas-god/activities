-- Migrate the sessions into the user database and tie them to the users table.
-- Existing sessions are not migrated for simplicity.
CREATE TABLE t_sessions (
    user INTEGER NOT NULL REFERENCES t_users (rowid) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,
    expire_at TIMESTAMP NOT NULL
);
