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

-- Seed pre-existing users with {push,discord} on event types where the
-- pre-dispatcher codebase unconditionally fired Busybee (Discord):
--   * your_turn  — turn_handler.rs, for correspondence games.
--   * challenges — challenges/accept.rs, for both players on
--                  correspondence/untimed accepts.
--   * tournament — tournaments/invitation_create.rs, on every invite.
-- Users who relied on either delivery would silently lose it once the
-- legacy calls were removed in favour of the unified dispatcher.
-- New users registered after this migration get the column defaults of
-- {push} — they can opt into Discord via /notifications when desired.
insert into notification_preferences (user_id, your_turn, challenges, tournament)
select
    id,
    array['push', 'discord']::text[],
    array['push', 'discord']::text[],
    array['push', 'discord']::text[]
from users;
