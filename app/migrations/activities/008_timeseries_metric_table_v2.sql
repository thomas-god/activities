
CREATE TABLE IF NOT EXISTS t_activities_metrics (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    metric TEXT,
    UNIQUE (metric)
);

CREATE TABLE IF NOT EXISTS t_activities_metrics_values (
    activity_rowid INTEGER,
    metric_rowid INTEGER,
    value REAL,
    FOREIGN KEY (activity_rowid) REFERENCES t_activities_v2(rowid) ON DELETE CASCADE,
    FOREIGN KEY (metric_rowid) REFERENCES t_activities_metrics(rowid) ON DELETE CASCADE,
    UNIQUE (activity_rowid, metric_rowid)
);
