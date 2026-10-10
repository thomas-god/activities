-- Merge auth link storage into the user database.
--
-- SQLite forbids foreign keys referencing the implicit rowid, so we have to recreate the table
-- with and explicit rowid column.
-- Migration done following https://sqlite.org/lang_altertable.html#making_other_kinds_of_table_schema_changes
CREATE TABLE t_users_new (
    rowid INTEGER PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    user_id TEXT UNIQUE NOT NULL
);

INSERT INTO
    t_users_new (email, user_id)
SELECT
    email,
    user_id
FROM
    t_users;

DROP TABLE t_users;

ALTER TABLE t_users_new RENAME TO t_users;

-- Auth links are now tied to the user's internal rowid. Existing links are short-lived and
-- disposable, so they are not migrated.
CREATE TABLE t_auth_links (
    user INTEGER NOT NULL REFERENCES t_users (rowid) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,
    expire_at TIMESTAMP NOT NULL
);
