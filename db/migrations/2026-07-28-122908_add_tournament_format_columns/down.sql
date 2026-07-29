drop index if exists tournaments_automated_in_progress;

alter table tournaments drop column third_place_match;
alter table tournaments drop column fully_automated;
