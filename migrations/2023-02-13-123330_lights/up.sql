-- Your SQL goes here
CREATE TABLE lights (
    light_id uuid PRIMARY KEY not NULL,
    rgb INT not NULL,
    brightness INT not NULL,
    is_on boolean not NULL,
    user_id INT not NUll,
    secret VARCHAR not NULL
);