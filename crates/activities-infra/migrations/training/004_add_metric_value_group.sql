-- Based on https://www.sqlite.org/lang_altertable.html
-- Transaction already started by the migration script
CREATE TABLE
    IF NOT EXISTS t_training_metrics_values_new (
        definition_id TEXT,
        granule TEXT,
        value BLOB,
        bin_group TEXT,
        FOREIGN KEY (definition_id) REFERENCES t_training_metrics_definitions (id) ON DELETE CASCADE,
        CONSTRAINT t_training_metrics_values_unique_id_granule_bin_group UNIQUE (definition_id, granule, bin_group) ON CONFLICT REPLACE
    );

INSERT INTO
    t_training_metrics_values_new
SELECT
    -- "no_group" for existing rows to match sqlite/training.rs::NONE_GROUP
    definition_id, granule, value, "no_group"
FROM
    t_training_metrics_values;

DROP TABLE t_training_metrics_values;

ALTER TABLE t_training_metrics_values_new
RENAME TO t_training_metrics_values;