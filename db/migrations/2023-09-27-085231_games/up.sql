create table games (
  id uuid default gen_random_uuid() primary key not null,
  url text not null,
  black_id uuid not null,
  game_status text not null,
  game_type text not null,
  history text not null,
  game_control_history text not null,
  rated boolean not null default true,
  tournament_queen_rule boolean not null default true,
  turn integer not null default 0,
  white_id uuid not null,
  white_rating float8,
  black_rating float8,
  white_rating_change float8,
  black_rating_change float8
);
