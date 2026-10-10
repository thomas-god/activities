-- Split the email into its own table and add a password hash column to t_users.
--
-- Rebuilding t_users requires dropping it (FK + UNIQUE constraint on dropped column)
-- Child tables are dropped and recreated (temp data, no migration), while emails are backed up
-- in their new table.

DROP TABLE t_auth_links;
DROP TABLE t_sessions;

CREATE TABLE t_emails_backup AS
SELECT
    email,
    rowid AS user
FROM
    t_users;

CREATE TABLE t_users_new (
    rowid INTEGER PRIMARY KEY,
    user_id TEXT UNIQUE NOT NULL,
    password_hash TEXT
);

INSERT INTO
    t_users_new (rowid, user_id)
SELECT
    rowid,
    user_id
FROM
    t_users;

DROP TABLE t_users;

ALTER TABLE t_users_new RENAME TO t_users;

CREATE TABLE t_auth_links (
    user INTEGER NOT NULL REFERENCES t_users (rowid) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,
    expire_at TIMESTAMP NOT NULL
);

CREATE TABLE t_sessions (
    user INTEGER NOT NULL REFERENCES t_users (rowid) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,
    expire_at TIMESTAMP NOT NULL
);

CREATE TABLE t_emails (
    email TEXT UNIQUE NOT NULL,
    user INTEGER NOT NULL REFERENCES t_users (rowid) ON DELETE CASCADE,
    verified INTEGER NOT NULL DEFAULT FALSE
);

-- Emails of existing users are considered verified.
INSERT INTO
    t_emails (email, user, verified)
SELECT
    email,
    user,
    TRUE
FROM
    t_emails_backup;

DROP TABLE t_emails_backup;
