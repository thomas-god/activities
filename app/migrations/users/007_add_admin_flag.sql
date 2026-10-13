-- Add the admin flag to users.
--
-- Existing instances had no concept of an admin, so every existing user starts as a non-admin. No
-- heuristic promotes anyone automatically: the first admin is established through the bootstrap
-- environment variables or the CLI (see the admin role ADR).
ALTER TABLE t_users ADD COLUMN is_admin INTEGER NOT NULL DEFAULT FALSE;
