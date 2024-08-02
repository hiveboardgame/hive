CREATE TABLE schedules (
  id uuid default gen_random_uuid() primary key not null,
  game_id uuid references games(id) on delete cascade not null,
  tournament_id uuid references tournaments(id) on delete cascade not null,
  proposer_id uuid references users(id) on delete cascade not null,
  opponent_id uuid references users(id) on delete cascade not null,
  start_t TIMESTAMP WITH TIME ZONE not null,
  agreed BOOLEAN NOT NULL DEFAULT false
);
