-- Your SQL goes here
CREATE TABLE lights (
    light_id uuid PRIMARY KEY not NULL,
    red INT not NULL,
    green INT not NULL,
    blue INT not null,
    brightness INT not NULL,
    is_on boolean not NULL,
    user_id INT not NUll,
    secret VARCHAR not NULL
);