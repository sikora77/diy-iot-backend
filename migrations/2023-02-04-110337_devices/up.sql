-- Your SQL goes here
CREATE TABLE devices (
    id uuid PRIMARY KEY,
    type VARCHAR NOT NULL,
    user_id INT NOT NULL,
    internal_name text NOT NULL,
    name text NOT NULL,
    nicknames text [] NOT NULL,
    traits text [] NOT NULL
)