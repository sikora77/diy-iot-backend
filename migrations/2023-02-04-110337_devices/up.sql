-- Your SQL goes here
CREATE TABLE devices (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    type VARCHAR NOT NULL,
    user_id INT NOT NULL,
    secret VARCHAR NOT NULL
)