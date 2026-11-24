CREATE TABLE
    IF NOT EXISTS t_training_periods (
        id TEXT UNIQUE,
        user_id TEXT,
        start TEXT,
        end TEXT NULLABLE,
        name TEXT,
        sports BLOB,
        note TEXT NULLABLE
    );
