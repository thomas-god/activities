-- Add feedback column to activities
-- Feedback is stored as free-form text
ALTER TABLE t_activities ADD COLUMN feedback TEXT;
