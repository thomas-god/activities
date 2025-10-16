CREATE TABLE
    IF NOT EXISTS t_magic_links (
        user TEXT,
        token_hash TEXT UNIQUE,
        expire_at TIMESTAMP
    );