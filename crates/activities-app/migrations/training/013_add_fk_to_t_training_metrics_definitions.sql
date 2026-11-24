CREATE TABLE t_training_metrics_definitions_v2 (
	id TEXT UNIQUE,
	user_id TEXT,
	source BLOB,
	granularity TEXT,
	aggregate TEXT,
	filters BLOB, 
	group_by BLOB, 
	name TEXT, 
	training_period_id TEXT, 
	activity_metric TEXT, 
	FOREIGN KEY (training_period_id) REFERENCES t_training_periods(id) on DELETE CASCADE
);

WITH metrics AS (
	SELECT * FROM t_training_metrics_definitions 
	WHERE 
		training_period_id IN (SELECT id FROM t_training_periods) 
		OR training_period_id IS NULL
)
INSERT INTO t_training_metrics_definitions_v2 SELECT * FROM metrics;

DROP TABLE IF EXISTS t_training_metrics_definitions;
ALTER TABLE t_training_metrics_definitions_v2 RENAME TO t_training_metrics_definitions;