create table users (
    uid text primary key not null,
    username text not null unique,
    password text not null,
    email text not null
)
