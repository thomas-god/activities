CREATE TABLE
    IF NOT EXISTS t_training_metrics_ordering (
        user_id TEXT NOT NULL,
        training_period_id TEXT,
        metric_ids TEXT NOT NULL
    );

-- Create a unique index that treats NULL as a distinct value for global scope
-- For global scope (training_period_id IS NULL), we want only one row per user
-- For period scope (training_period_id IS NOT NULL), we want unique (user_id, training_period_id)
CREATE UNIQUE INDEX idx_training_metrics_ordering_user_global
    ON t_training_metrics_ordering (user_id)
    WHERE training_period_id IS NULL;

CREATE UNIQUE INDEX idx_training_metrics_ordering_user_period
    ON t_training_metrics_ordering (user_id, training_period_id)
    WHERE training_period_id IS NOT NULL;
