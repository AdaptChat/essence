CREATE TABLE message_references (
    target_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE, -- The target message that this message is referencing.
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL,
    guild_id BIGINT,
    mention_author BOOLEAN NOT NULL DEFAULT FALSE
);
