alter table users add column email_verified bool not null default false;
-- pending_email is intentionally not unique: a unique index would let an attacker
-- probe whether an address is already in use by attempting to set it as pending.
-- Collisions are caught at verify time by the existing unique constraint on email.
alter table users add column pending_email text;
alter table users add column locale text not null default 'en';
alter table users add column notifications_enabled bool not null default true;
alter table users add column notify_only_when_offline bool not null default true;
alter table users add column notification_mode text not null default 'digest';
