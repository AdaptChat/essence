CREATE TABLE IF NOT EXISTS messages (
    id BIGINT NOT NULL PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    type TEXT NOT NULL DEFAULT 'default',
    content TEXT,
    embeds JSONB NOT NULL DEFAULT '[]'::JSONB,
    flags INTEGER NOT NULL DEFAULT 0,
    stars INTEGER NOT NULL DEFAULT 0,
    metadata_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    metadata_pinned_message_id BIGINT,
    metadata_pinned_by BIGINT REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (metadata_pinned_message_id)
        REFERENCES messages(id)
        ON DELETE SET NULL
);