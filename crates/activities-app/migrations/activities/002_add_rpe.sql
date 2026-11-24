-- Add RPE (Relative Perceived Exertion) column to activities
-- RPE is an optional integer from 1 to 10
ALTER TABLE t_activities ADD COLUMN rpe INTEGER NULLABLE;
