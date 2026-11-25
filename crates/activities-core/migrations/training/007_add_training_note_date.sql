-- Add date column to training notes
-- This represents the domain date (when the note is about) vs created_at (when it was created)
ALTER TABLE t_training_notes
ADD COLUMN date TEXT;
