-- Drop definitions with deprecated sources that can't be parsed
DELETE FROM t_training_metrics_definitions
WHERE activity_metric IN (
    "wn-fat",
    "wn-muscle",
    "wn-bmi",
    "wn-lipid",
    "wn-carbs",
    "wn-protein"
);
