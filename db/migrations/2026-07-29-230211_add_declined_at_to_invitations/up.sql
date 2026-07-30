-- Declining used to delete the invitation row, which threw away the fact that
-- anybody had declined at all. An organizer needs to see who has said no —
-- otherwise a silent decline is indistinguishable from an invitation nobody has
-- looked at yet.
alter table tournaments_invitations add column declined_at timestamptz;
