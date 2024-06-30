create table tournaments_organizers (
  tournament_id uuid references tournaments(id) on delete cascade,
  organizer_id uuid references users(id),
  primary key(tournament_id, organizer_id)
);
