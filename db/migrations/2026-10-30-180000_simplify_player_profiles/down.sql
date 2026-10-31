-- Rollback restores the fields, but cannot recover their deliberately deleted values.
ALTER TABLE player_profiles
    ADD COLUMN location TEXT CHECK (location IS NULL OR char_length(location) <= 150),
    ADD COLUMN birth_year INTEGER CHECK (birth_year IS NULL OR birth_year >= 1900),
    ADD COLUMN gender TEXT CHECK (gender IS NULL OR gender IN ('M', 'F', 'X', '?'));
