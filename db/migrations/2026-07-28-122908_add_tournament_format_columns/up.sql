alter table tournaments add column fully_automated boolean not null default false;
alter table tournaments add column third_place_match boolean not null default false;

-- The progress job scans for exactly this pair on every tick, and automated
-- tournaments are a small slice of the table.
create index tournaments_automated_in_progress
    on tournaments (status)
    where fully_automated;
