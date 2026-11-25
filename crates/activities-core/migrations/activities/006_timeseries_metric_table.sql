CREATE TABLE IF NOT EXISTS t_activities_timeseries_metrics (
    activity TEXT,
    metric TEXT,
    aggregate TEXT,
    found BOOLEAN,
    value REAL,
    FOREIGN KEY (activity) REFERENCES t_activities(id) ON DELETE CASCADE,
    UNIQUE (activity, metric, aggregate)
);

CREATE INDEX IF NOT EXISTS t_activities_timeseries_metrics_idx
ON t_activities_timeseries_metrics(activity, metric, aggregate);