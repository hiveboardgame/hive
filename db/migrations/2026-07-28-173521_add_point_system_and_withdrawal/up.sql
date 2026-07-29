-- Every value of the point system, per tournament. Null means "use the
-- default for this mode", which is how existing rows keep their behaviour:
-- 1 / ½ / 0, a forfeit worth nothing, and a bye worth a full point in Swiss
-- but nothing in a round robin. Stored in human units (1.0, 0.5) rather than
-- the engine's doubled integers.
alter table tournaments add column points_win float8;
alter table tournaments add column points_draw float8;
alter table tournaments add column points_loss float8;
alter table tournaments add column points_forfeit_loss float8;
alter table tournaments add column points_zero_point_bye float8;
alter table tournaments add column points_pairing_allocated_bye float8;

-- Leaving one tournament, as opposed to deleting the whole account. Nullable
-- and clearable, so an organizer can reinstate somebody who withdrew by
-- mistake.
alter table tournaments_users add column withdrawn_at timestamptz;
