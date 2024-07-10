create table tournaments (
  id uuid default gen_random_uuid() primary key not null, -- postgresql id
  nanoid text unique not null, -- short url
  name text not null unique, -- name of the tournament
  description text not null, -- a description of the tournament
  scoring text not null, -- per match or per game scoring
  tiebreaker text[] not null default '{}', -- list of tiebreakers
  seats int not null, -- maximum number of players
  min_seats int not null, -- minimum number of players
  rounds int not null, -- Number of RR games, total number of SWISS games
  invite_only bool not null default false, -- can players join the tournament without an invite
  mode text not null, -- RR, SWISS, ...
  time_mode text not null, -- either 'Timed' or 'Correspondence',
  time_base int, -- seconds
  time_increment int, -- seconds
  band_upper int, -- max elo
  band_lower int, -- min elo
  -- either when does the tournament start for tournaments with a start date
  -- or when did it start for tournaments that start when enough players signed up
  start_mode text not null,
  starts_at timestamp with time zone, -- when will the tournaments start, for automated tournaments
  ends_at timestamp with time zone, -- when will the tournaments end, for
  -- manual tournaments it's just to show a date to the user and for automated
  -- tournaments this will be used to end the tournament at.
  started_at timestamp with time zone,  -- when did the tournaments start
  round_duration int, -- how long does a round run for in days
  status text not null,
  created_at timestamp with time zone not null, -- when was it created
  updated_at timestamp with time zone not null -- when was the last update made to the model
)
