create table notification_preferences (
  user_id      uuid primary key references users(id) on delete cascade,
  your_turn    text[] not null default '{push}',
  challenges   text[] not null default '{push}',
  game_ended   text[] not null default '{push}',
  tournament   text[] not null default '{push}',
  general_chat text[] not null default '{}',
  dms          text[] not null default '{push}',
  quiet_start  smallint,
  quiet_end    smallint,
  timezone     text,
  constraint notification_preferences_channels_valid check (
    your_turn    <@ array['push','email','discord']::text[] and
    challenges   <@ array['push','email','discord']::text[] and
    game_ended   <@ array['push','email','discord']::text[] and
    tournament   <@ array['push','email','discord']::text[] and
    general_chat <@ array['push','email','discord']::text[] and
    dms          <@ array['push','email','discord']::text[]
  ),
  constraint notification_preferences_quiet_hours_valid check (
    (quiet_start is null and quiet_end is null)
    or
    (quiet_start is not null and quiet_end is not null
     and quiet_start between 0 and 23
     and quiet_end between 0 and 23)
  )
);

-- Seed pre-existing users with {push,discord} on `your_turn`. The codebase
-- before this migration unconditionally fired Busybee (Discord) from
-- turn_handler.rs for correspondence games, so users who relied on that
-- delivery would silently lose it once the legacy call was removed in
-- favour of the unified dispatcher. New users registered after this
-- migration get the column default of {push} — they can opt into Discord
-- via the settings UI when that lands.
insert into notification_preferences (user_id, your_turn)
select id, array['push', 'discord']::text[] from users;
