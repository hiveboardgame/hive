-- Local development testware. Edit this file to evolve the seeded data.
--
-- Login credentials:
--   admin_1 / password  (administrator)
--   user_1  / password
--   user_2  / password
--
-- The UUIDs are stable so future fixture rows can safely reference these users.
-- The shared password uses the fixed Argon2id hash below and is for local
-- development only.

DO $$
DECLARE
    statement text;
BEGIN
    SELECT 'TRUNCATE TABLE ' ||
        string_agg(format('%I.%I', schemaname, tablename), ', ') ||
        ' RESTART IDENTITY CASCADE'
    INTO statement
    FROM pg_tables
    WHERE schemaname = 'public'
        AND tablename <> '__diesel_schema_migrations';

    IF statement IS NOT NULL THEN
        EXECUTE statement;
    END IF;
END $$;

INSERT INTO users (
    id,
    username,
    password,
    email,
    created_at,
    updated_at,
    normalized_username,
    admin,
    email_verified
)
VALUES
    (
        '00000000-0000-4000-8000-000000000001',
        'admin_1',
        '$argon2id$v=19$m=19456,t=2,p=1$aGl2ZS10ZXN0d2FyZS12MQ$nfYuD8uwlN2TrxGZnNptJufwSu7LQ2IN/ns/LoJ6TzI',
        'admin_1@example.test',
        now(),
        now(),
        'admin_1',
        true,
        true
    ),
    (
        '00000000-0000-4000-8000-000000000002',
        'user_1',
        '$argon2id$v=19$m=19456,t=2,p=1$aGl2ZS10ZXN0d2FyZS12MQ$nfYuD8uwlN2TrxGZnNptJufwSu7LQ2IN/ns/LoJ6TzI',
        'user_1@example.test',
        now(),
        now(),
        'user_1',
        false,
        true
    ),
    (
        '00000000-0000-4000-8000-000000000003',
        'user_2',
        '$argon2id$v=19$m=19456,t=2,p=1$aGl2ZS10ZXN0d2FyZS12MQ$nfYuD8uwlN2TrxGZnNptJufwSu7LQ2IN/ns/LoJ6TzI',
        'user_2@example.test',
        now(),
        now(),
        'user_2',
        false,
        true
    );

-- Match User::create: one initial rating for each rated game speed.
INSERT INTO ratings (
    user_uid,
    played,
    won,
    lost,
    draw,
    rating,
    deviation,
    volatility,
    created_at,
    updated_at,
    speed
)
SELECT
    seeded_users.id,
    0,
    0,
    0,
    0,
    1500.0,
    500.0,
    0.09,
    now(),
    now(),
    speeds.speed
FROM (
    VALUES
        ('00000000-0000-4000-8000-000000000001'::uuid),
        ('00000000-0000-4000-8000-000000000002'::uuid),
        ('00000000-0000-4000-8000-000000000003'::uuid)
) AS seeded_users(id)
CROSS JOIN (
    VALUES
        ('Bullet'),
        ('Blitz'),
        ('Rapid'),
        ('Classic'),
        ('Correspondence'),
        ('Puzzle')
) AS speeds(speed);

INSERT INTO notification_preferences (user_id)
VALUES
    ('00000000-0000-4000-8000-000000000001'),
    ('00000000-0000-4000-8000-000000000002'),
    ('00000000-0000-4000-8000-000000000003');

-- These singleton rows are inserted by migrations but are removed by a full
-- truncate. Restore them so the admin banner and email maintenance work.
INSERT INTO home_banner (title, content, display)
VALUES (
    'Welcome to Hive!',
    'This is the default banner for new users. You can edit it in the admin panel.',
    false
);

INSERT INTO email_state (id)
VALUES (1);
