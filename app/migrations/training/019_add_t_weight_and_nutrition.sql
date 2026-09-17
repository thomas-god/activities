CREATE TABLE IF NOT EXISTS t_weight_and_nutrition (
    rowid    INTEGER PRIMARY KEY AUTOINCREMENT,
    user     TEXT NOT NULL,
    date     TEXT NOT NULL,
    weight   REAL,
    fat      REAL,
    muscle   REAL,
    bmi      REAL,
    calories REAL,
    lipid    REAL,
    carbs    REAL,
    protein  REAL,
    water    REAL,
    alcohol  REAL,
    UNIQUE(user, date)
);
