CREATE TABLE IF NOT EXISTS channel_acks (
    channel_id BIGINT NOT NULL REFERENCES channels (id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    last_message_id BIGINT,
    PRIMARY KEY (channel_id, user_id)
);