create table challenges (
  id uuid default gen_random_uuid() primary key not null,
  nanoid text not null,
  challenger_id uuid references users(id) not null,
  opponent_id uuid references users(id),
  game_type text not null,
  rated boolean not null,
  visibility text not null,
  tournament_queen_rule boolean not null,
  color_choice text not null,
  created_at timestamp with time zone not null
)
