-- All fixture-owned records reference these users and use ON DELETE CASCADE.
DELETE FROM users
WHERE id IN (
    SELECT ('00000000-0000-4000-8000-' || lpad(to_hex(n), 12, '0'))::uuid
    FROM generate_series(1, 9) AS n
);
