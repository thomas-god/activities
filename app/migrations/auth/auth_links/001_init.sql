CREATE TABLE
    IF NOT EXISTS t_auth_links (
        user TEXT,
        token_hash TEXT UNIQUE,
        expire_at TIMESTAMP
    );