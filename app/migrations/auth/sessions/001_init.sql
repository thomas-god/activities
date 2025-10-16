CREATE TABLE
    IF NOT EXISTS t_sessions (
        user TEXT,
        token_hash TEXT UNIQUE,
        expire_at TIMESTAMP
    );