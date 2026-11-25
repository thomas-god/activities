CREATE TABLE
    IF NOT EXISTS t_training_metrics_definitions (
        id TEXT UNIQUE,
        user_id TEXT,
        source BLOB,
        granularity TEXT,
        aggregate TEXT,
        filters BLOB
    );

CREATE TABLE
    IF NOT EXISTS t_training_metrics_values (
        definition_id TEXT,
        granule TEXT,
        value BLOB,
        FOREIGN KEY (definition_id) REFERENCES t_training_metrics_definitions (id) ON DELETE CASCADE,
        CONSTRAINT t_training_metrics_values_unique_id_granule UNIQUE (definition_id, granule) ON CONFLICT REPLACE
    );