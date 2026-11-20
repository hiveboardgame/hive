CREATE TABLE player_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    flag TEXT,
    bio TEXT NOT NULL DEFAULT '' CHECK (char_length(bio) <= 300),
    bio_restricted BOOLEAN NOT NULL DEFAULT FALSE
);
