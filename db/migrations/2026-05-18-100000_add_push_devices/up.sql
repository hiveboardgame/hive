CREATE TABLE push_devices (
  id uuid default gen_random_uuid() primary key not null,
  user_id uuid references users(id) on delete cascade not null,
  platform TEXT NOT NULL CHECK (platform IN ('apns', 'fcm')),
  device_token TEXT NOT NULL,
  app_version TEXT NOT NULL,
  locale TEXT NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
  last_seen_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
  UNIQUE (platform, device_token)
);

CREATE INDEX push_devices_user_id_idx ON push_devices(user_id);
