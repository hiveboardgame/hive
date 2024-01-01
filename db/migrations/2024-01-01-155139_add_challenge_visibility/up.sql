alter table challenges add column opponent_id uuid references users(id);
alter table challenges add column visibility text not null default 'Public';
alter table challenges drop column public;
