CREATE TABLE IF NOT EXISTS mentions (
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    target_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    PRIMARY KEY (message_id, target_id)
);