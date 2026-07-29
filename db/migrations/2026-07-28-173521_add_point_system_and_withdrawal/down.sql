alter table tournaments_users drop column withdrawn_at;

alter table tournaments drop column points_pairing_allocated_bye;
alter table tournaments drop column points_zero_point_bye;
alter table tournaments drop column points_forfeit_loss;
alter table tournaments drop column points_loss;
alter table tournaments drop column points_draw;
alter table tournaments drop column points_win;
