-- Your SQL goes here
CREATE TABLE home_banner (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    display BOOLEAN NOT NULL DEFAULT FALSE
);

INSERT INTO home_banner (title, content, display)
VALUES ('Welcome to Hive!', 'This is the default banner for new users. You can edit it in the admin panel.', FALSE);
