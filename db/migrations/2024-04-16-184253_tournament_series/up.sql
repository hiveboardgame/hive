create table tournament_series (
  id uuid default gen_random_uuid() primary key not null, -- postgresql id
  nanoid text unique not null, -- short url
  name text not null, -- name of the tournament
  description text not null, -- some info about the tournament
  created_at timestamp with time zone not null, -- when was it created
  updated_at timestamp with time zone not null -- when was the last update made to the model
)
