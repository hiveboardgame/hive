create table notification_preferences (
  user_id      uuid primary key references users(id) on delete cascade,
  your_turn    text[] not null default '{push}',
  challenges   text[] not null default '{push}',
  game_ended   text[] not null default '{push}',
  tournament   text[] not null default '{push}',
  schedules    text[] not null default '{push}',
  general_chat text[] not null default '{}',
  dms          text[] not null default '{push}',
  constraint notification_preferences_channels_valid check (
    your_turn    <@ array['push','email','discord']::text[] and
    challenges   <@ array['push','email','discord']::text[] and
    game_ended   <@ array['push','email','discord']::text[] and
    tournament   <@ array['push','email','discord']::text[] and
    schedules    <@ array['push','email','discord']::text[] and
    general_chat <@ array['push','email','discord']::text[] and
    dms          <@ array['push','email','discord']::text[]
  )
);

insert into notification_preferences (user_id, your_turn, challenges, tournament, schedules)
select
    id,
    array['push', 'discord']::text[],
    array['push', 'discord']::text[],
    array['push', 'discord']::text[],
    array['push', 'discord']::text[]
from users;
