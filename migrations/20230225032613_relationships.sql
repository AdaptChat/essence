CREATE TYPE relationship_type AS ENUM ('friend', 'blocked', 'pending_otn', 'pending_nto');

CREATE TABLE IF NOT EXISTS relationships (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    other_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type relationship_type NOT NULL,
    PRIMARY KEY (user_id, other_id)
);
