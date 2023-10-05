create table challenges (
  id uuid default gen_random_uuid() primary key not null,
  url text not null,
  challenger_id uuid references users(id) not null,
  game_type text not null,
  rated boolean not null,
  public boolean not null,
  tournament_queen_rule boolean not null,
  color_choice text not null,
  created_at timestamp with time zone not null
)
