create table email_state (
  id smallint primary key default 1 check (id = 1),
  last_cleanup_run_at timestamp with time zone
);

insert into email_state (id) values (1);
