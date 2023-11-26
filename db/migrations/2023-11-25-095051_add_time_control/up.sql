CREATE TYPE time_control AS ENUM ('untimed', 'correspondence', 'real_time');
ALTER TABLE challenges ADD COLUMN timer time_control NOT NULL DEFAULT time_control('untimed');