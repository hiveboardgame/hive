drop index tournaments_users_seed;

alter table tournaments_users drop column rating;
alter table tournaments_users drop column seed;
