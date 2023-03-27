CREATE TYPE relationship_type AS ENUM (
    'friend',
    'blocked', -- user blocked target
    'incoming', -- target has sent a friend request to user
    'outgoing' -- user has sent a friend request to target
);

CREATE TABLE IF NOT EXISTS relationships (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type relationship_type NOT NULL,
    PRIMARY KEY (user_id, target_id)
);
