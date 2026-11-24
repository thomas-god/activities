-- Tokens used to verify a change of email address for an existing user.

CREATE TABLE t_email_change_tokens (
    user INTEGER NOT NULL REFERENCES t_users (rowid) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,
    new_email TEXT NOT NULL,
    expire_at TIMESTAMP NOT NULL
);
