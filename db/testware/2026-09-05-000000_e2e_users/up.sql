-- End-to-end fixture accounts. These values are intentionally stable so test
-- data can reference them. The password for every account is `password`.
--
-- This migration is kept outside db/migrations so application deployments do
-- not receive fixture data. Apply it explicitly with:
--   diesel migration run --migration-dir testware

-- Preserve the original IDs: admin_1 is 1, user_1 is 2, ..., user_8 is 9.
WITH fixture_accounts AS (
    SELECT
        ('00000000-0000-4000-8000-' || lpad(to_hex(n + 1), 12, '0'))::uuid AS id,
        CASE WHEN n = 0 THEN 'admin_1' ELSE 'user_' || n END AS username,
        n = 0 AS admin
    FROM generate_series(0, 8) AS n
)
INSERT INTO users (
    id, username, password, email, created_at, updated_at,
    normalized_username, admin, email_verified
)
SELECT
    id,
    username,
    '$argon2id$v=19$m=19456,t=2,p=1$aGl2ZS10ZXN0d2FyZS12MQ$nfYuD8uwlN2TrxGZnNptJufwSu7LQ2IN/ns/LoJ6TzI',
    username || '@example.test',
    now(), now(), username, admin, true
FROM fixture_accounts;

-- Match User::create: one initial rating for each rated game speed.
INSERT INTO ratings (
    user_uid, played, won, lost, draw, rating, deviation, volatility,
    created_at, updated_at, speed
)
SELECT
    ('00000000-0000-4000-8000-' || lpad(to_hex(n), 12, '0'))::uuid,
    0, 0, 0, 0, 1500.0, 500.0, 0.09, now(), now(), speeds.speed
FROM generate_series(1, 9) AS n
CROSS JOIN (
    VALUES ('Bullet'), ('Blitz'), ('Rapid'), ('Classic'), ('Correspondence'), ('Puzzle')
) AS speeds(speed);

INSERT INTO notification_preferences (user_id)
SELECT ('00000000-0000-4000-8000-' || lpad(to_hex(n), 12, '0'))::uuid
FROM generate_series(1, 9) AS n;
