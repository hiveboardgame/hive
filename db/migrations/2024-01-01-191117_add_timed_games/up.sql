alter table games add column time_mode text not null default 'Untimed'; -- 'Timed', 'Correspondence',
alter table games add column time_base int; -- seconds
alter table games add column time_increment int; -- seconds
alter table games add column last_interaction timestamp with time zone;
alter table games add column black_time_left bigint;
alter table games add column white_time_left bigint;

alter table challenges add column time_mode text not null default 'Untimed'; -- 'Timed', 'Correspondence',
alter table challenges add column time_base int; -- seconds
alter table challenges add column time_increment int; -- seconds
