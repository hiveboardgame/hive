create table email_state (
  id smallint primary key default 1 check (id = 1),
  last_digest_run_at timestamp with time zone,
  last_cleanup_run_at timestamp with time zone
);

insert into email_state (id) values (1);
