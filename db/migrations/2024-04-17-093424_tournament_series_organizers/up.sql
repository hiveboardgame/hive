create table tournament_series_organizers (
  tournament_series_id uuid references tournament_series(id) on delete cascade,
  organizer_id uuid references users(id),
  primary key(tournament_series_id, organizer_id)
);
