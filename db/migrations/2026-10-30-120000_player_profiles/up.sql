CREATE TABLE player_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    flag TEXT CHECK (flag IS NULL OR flag ~ '^[A-Z]{2}$'),
    location TEXT CHECK (location IS NULL OR char_length(location) <= 150),
    birth_year INTEGER CHECK (birth_year IS NULL OR birth_year >= 1900),
    gender TEXT CHECK (gender IS NULL OR gender IN ('M', 'F', 'X', '?')),
    bio TEXT NOT NULL DEFAULT '' CHECK (char_length(bio) <= 300)
);
