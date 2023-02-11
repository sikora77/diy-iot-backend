-- Your SQL goes here
CREATE TABLE devices (
    id SERIAL PRIMARY KEY,
    type VARCHAR NOT NULL,
    user_id INT NOT NULL
)