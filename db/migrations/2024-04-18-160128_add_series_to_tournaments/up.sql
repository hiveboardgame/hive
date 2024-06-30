alter table tournaments add column series uuid references tournament_series(id);
