alter table users add column email_verified bool not null default false;
-- Grandfather existing accounts so we don't lock them out by suddenly requiring
-- verification; new rows default to false.
update users set email_verified = true;
-- pending_email is intentionally not unique: a unique index would let an attacker
-- probe whether an address is already in use by attempting to set it as pending.
-- Collisions are caught at verify time by the existing unique constraint on email.
alter table users add column pending_email text;
