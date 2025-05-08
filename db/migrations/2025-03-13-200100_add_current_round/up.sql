-- Add current_round column to tournaments table
ALTER TABLE tournaments ADD COLUMN current_round integer NOT NULL DEFAULT 1; 