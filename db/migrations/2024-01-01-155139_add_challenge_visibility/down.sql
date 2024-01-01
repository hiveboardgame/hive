alter table challenges drop column opponent_id;
alter table challenges drop column visibility;
alter table challenges add column public boolean not null default true;
