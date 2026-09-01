create index ratings_speed_rating on ratings (speed, rating desc);
create index users_bot on users (id) where bot;
