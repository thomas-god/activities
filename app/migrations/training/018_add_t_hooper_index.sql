CREATE TABLE IF NOT EXISTS t_hooper_index (
    rowid   INTEGER PRIMARY KEY AUTOINCREMENT,
    user    TEXT NOT NULL,
    date    TEXT NOT NULL,
    fatigue INTEGER,
    sleep   INTEGER,
    pain    INTEGER,
    stress  INTEGER,
    mood    INTEGER,
    UNIQUE(user, date)
);
