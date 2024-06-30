create table tournaments_invitations (
  tournament_id uuid references tournaments(id) on delete cascade,
  invitee_id uuid references users(id),
  primary key(tournament_id, invitee_id),
  created_at timestamp with time zone not null
);
