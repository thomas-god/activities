-- Bind each authentication link to the address it was sent to.

ALTER TABLE t_auth_links ADD COLUMN email TEXT;
