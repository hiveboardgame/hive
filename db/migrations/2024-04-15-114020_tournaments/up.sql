create table tournaments (
  id uuid default gen_random_uuid() primary key not null, -- postgresql id
  nanoid text unique not null, -- short url
  name text not null, -- name of the tournament
  description text not null, -- a description of the tournament
  scoring text not null, -- per match or per game scoring
  tiebreaker text[] not null default '{}', -- list of tiebreakers
  invitees uuid[] not null default '{}', -- invited players
  seats int not null, -- maximum number of players
  rounds int not null, -- Number of RR games, total number of SWISS games
  joinable bool not null default true, -- this means the tournament has started and people cannot join it anymore
  invite_only bool not null default false, -- can players join the tournament without an invite?
  mode text not null, -- RR, SWISS, ...
  time_mode text not null, -- either 'Timed' or 'Correspondence',
  time_base int, -- seconds
  time_increment int, -- seconds
  band_upper int, -- max elo
  band_lower int, -- min elo
  -- either when does the tournament start for tournaments with a start date
  -- or when did it start for tournaments that start when enough players signed up
  -- TODO: @leex start_mode text not null,
  start_at timestamp with time zone, 
  created_at timestamp with time zone not null, -- when was it created
  updated_at timestamp with time zone not null -- when was the last update made to the model
)
