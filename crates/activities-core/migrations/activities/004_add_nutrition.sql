-- Add nutrition column to activities
-- Nutrition is stored as a JSON blob containing bonk status and details
ALTER TABLE t_activities ADD COLUMN nutrition BLOB;
